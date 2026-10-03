// crates/client/src/admin/services/offline_queue.rs
use crate::shared::services::queue_core::{QueueConfig, QueueCore, QueuePriority};
#[cfg(target_arch = "wasm32")]
use crate::shared::services::queue_core::QueueAction;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdminAction {
    BulkUserUpdate,
    ImportCsv,
    UpdateCourseMetadata,
    RevokeLicense,
    DraftAnnouncement,
}

#[derive(Clone)]
pub struct AdminQueue {
    core: QueueCore<AdminAction>,
}

impl AdminQueue {
    pub fn new() -> Self {
        Self {
            core: QueueCore::new(QueueConfig {
                storage_key: "admin_offline_queue".to_string(),
                max_retries: 10,
                queue_name: "admin".to_string(),
            }),
        }
    }

    pub fn enqueue_bulk_update(&self, user_ids: Vec<String>, changes: serde_json::Value, queue_size_signal: WriteSignal<usize>) {
        self.core.enqueue(
            AdminAction::BulkUserUpdate,
            serde_json::json!({
                "user_ids": user_ids,
                "changes": changes,
                "requires_rollback": true
            }),
            QueuePriority::High,
        );
        queue_size_signal.set(self.core.len());
    }

    pub fn enqueue_csv_import(&self, file_id: String, mapping: serde_json::Value, queue_size_signal: WriteSignal<usize>) {
        self.core.enqueue(
            AdminAction::ImportCsv,
            serde_json::json!({
                "file_id": file_id,
                "mapping": mapping,
                "atomic": true
            }),
            QueuePriority::Normal,
        );
        queue_size_signal.set(self.core.len());
    }

    pub fn enqueue_draft_announcement(&self, content: String, target_audience: String, queue_size_signal: WriteSignal<usize>) {
        self.core.enqueue(
            AdminAction::DraftAnnouncement,
            serde_json::json!({
                "content": content,
                "target_audience": target_audience
            }),
            QueuePriority::Low,
        );
        queue_size_signal.set(self.core.len());
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn process_queue(&self, queue_size_signal: WriteSignal<usize>) {
        use crate::shared::storage::get_storage;

        const CHUNK_SIZE: usize = 50;

        loop {
            let storage = match get_storage().await {
                Ok(s) => s,
                Err(_) => break,
            };

            let statements = match storage.get_statements_chunked(CHUNK_SIZE).await {
                Ok(stmts) => stmts,
                Err(_) => break,
            };

            if statements.is_empty() {
                break;
            }

            let chunk: Vec<_> = statements.into_iter().filter(|s| s.queue_name == "admin").collect();
            
            if chunk.is_empty() {
                break;
            }

            let chunk_ids: Vec<String> = chunk.iter().map(|s| s.id.clone()).collect();

            leptos::logging::log!(
                "[AdminQueue] Syncing chunk of {} statements...",
                chunk_ids.len()
            );

            let api_result = mock_sync_api(&chunk).await;

            match api_result {
                Ok(synced_ids) => {
                    leptos::logging::log!("[AdminQueue] Chunk synced successfully. Deleting from DB...");
                    if storage.delete_statements(&synced_ids).await.is_ok() {
                        leptos::logging::log!("[AdminQueue] Deleted {} statements from DB", synced_ids.len());
                    }
                }
                Err(e) => {
                    leptos::logging::warn!("[AdminQueue] Sync failed: {}. Retrying later.", e);
                    
                    for mut stmt in chunk {
                        if stmt.retry_count < 10 {
                            stmt.retry_count += 1;
                            let _ = storage.save_statement(&stmt).await;
                        } else {
                            leptos::logging::error!("[AdminQueue] Statement {} exceeded max retries, discarding", stmt.id);
                            let _ = storage.delete_statements(&[stmt.id]).await;
                        }
                    }
                    break;
                }
            }

            let remaining = storage.get_statements_chunked(1000).await.map(|s| {
                s.into_iter().filter(|x| x.queue_name == "admin").count()
            }).unwrap_or(0);
            queue_size_signal.set(remaining);
        }
        
        let storage = get_storage().await.ok();
        if let Some(s) = storage {
            let final_count = s.get_statements_chunked(1000).await.map(|s| {
                s.into_iter().filter(|x| x.queue_name == "admin").count()
            }).unwrap_or(0);
            queue_size_signal.set(final_count);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn process_queue(&self, _queue_size_signal: WriteSignal<usize>) {}

    #[cfg(target_arch = "wasm32")]
    pub async fn restore_from_storage(&self, queue_size_signal: WriteSignal<usize>) {
        use crate::shared::storage::get_storage;

        if let Ok(storage) = get_storage().await {
            if let Ok(statements) = storage.get_statements_chunked(1000).await {
                let mut queue = self.core.queue.borrow_mut();
                queue.clear();
                let mut count = 0;

                for stmt in statements {
                    if stmt.queue_name == "admin" {
                        let action_type = match stmt.verb.as_str() {
                            "BulkUserUpdate" => AdminAction::BulkUserUpdate,
                            "ImportCsv" => AdminAction::ImportCsv,
                            "UpdateCourseMetadata" => AdminAction::UpdateCourseMetadata,
                            "RevokeLicense" => AdminAction::RevokeLicense,
                            "DraftAnnouncement" => AdminAction::DraftAnnouncement,
                            _ => continue,
                        };

                        count += 1;
                        
                        let created_at = chrono::DateTime::parse_from_rfc3339(&stmt.timestamp)
                            .map(|dt| dt.timestamp())
                            .unwrap_or(0);

                        queue.push_back(QueueAction {
                            id: stmt.id,
                            action_type,
                            payload: serde_json::json!({ "object": stmt.object }),
                            created_at,
                            retry_count: stmt.retry_count,
                            priority: QueuePriority::Normal,
                        });
                    }
                }
                queue_size_signal.set(count);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn restore_from_storage(&self, _queue_size_signal: WriteSignal<usize>) {}

    pub fn len(&self) -> usize {
        self.core.len()
    }
}

thread_local! {
    pub static ADMIN_QUEUE: AdminQueue = AdminQueue::new();
}

#[cfg(target_arch = "wasm32")]
async fn mock_sync_api(statements: &[crate::shared::storage::XapiStatement]) -> Result<Vec<String>, String> {
    gloo::timers::future::TimeoutFuture::new(800).await;
    let confirmed_ids: Vec<String> = statements.iter().map(|s| s.id.clone()).collect();
    Ok(confirmed_ids)
}