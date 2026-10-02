// crates/client/src/student/services/offline_queue.rs
use crate::shared::services::queue_core::{QueueAction, QueueConfig, QueueCore, QueuePriority};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StudentAction {
    SubmitProgress,
    CompleteLesson,
    SubmitQuiz,
    SyncBookmark,
}

#[derive(Clone)]
pub struct StudentQueue {
    core: QueueCore<StudentAction>,
}

impl StudentQueue {
    pub fn new() -> Self {
        Self {
            core: QueueCore::new(QueueConfig {
                storage_key: "student_offline_queue".to_string(),
                max_retries: 3,
                queue_name: "student".to_string(),
            }),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn enqueue_progress(&self, course_id: &str, progress: u8, queue_size_signal: WriteSignal<usize>) {
        self.core.enqueue(
            StudentAction::SubmitProgress,
            serde_json::json!({ "course_id": course_id, "progress": progress }),
            QueuePriority::Critical,
        );
        queue_size_signal.set(self.core.len());
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn enqueue_progress(&self, _course_id: &str, _progress: u8, _queue_size_signal: WriteSignal<usize>) {}

    #[cfg(target_arch = "wasm32")]
    pub async fn process_queue(&self, queue_size_signal: WriteSignal<usize>) {
        use crate::shared::storage::get_storage;
        use gloo::timers::future::TimeoutFuture;

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

            let chunk: Vec<_> = statements.into_iter().filter(|s| s.queue_name == "student").collect();
            
            if chunk.is_empty() {
                break;
            }

            let chunk_ids: Vec<String> = chunk.iter().map(|s| s.id.clone()).collect();

            leptos::logging::log!(
                "[StudentQueue] Syncing chunk of {} statements...",
                chunk_ids.len()
            );

            let api_result = mock_sync_api(&chunk).await;

            match api_result {
                Ok(synced_ids) => {
                    leptos::logging::log!("[StudentQueue] Chunk synced successfully. Deleting from DB...");
                    if storage.delete_statements(&synced_ids).await.is_ok() {
                        leptos::logging::log!("[StudentQueue] Deleted {} statements from DB", synced_ids.len());
                    }
                }
                Err(e) => {
                    leptos::logging::warn!("[StudentQueue] Sync failed: {}. Retrying later.", e);
                    
                    for mut stmt in chunk {
                        if stmt.retry_count < 3 {
                            stmt.retry_count += 1;
                            let _ = storage.save_statement(&stmt).await;
                        } else {
                            leptos::logging::error!("[StudentQueue] Statement {} exceeded max retries, discarding", stmt.id);
                            let _ = storage.delete_statements(&[stmt.id]).await;
                        }
                    }
                    break;
                }
            }

            let remaining = storage.get_statements_chunked(1000).await.map(|s| {
                s.into_iter().filter(|x| x.queue_name == "student").count()
            }).unwrap_or(0);
            queue_size_signal.set(remaining);
        }
        
        let storage = get_storage().await.ok();
        if let Some(s) = storage {
            let final_count = s.get_statements_chunked(1000).await.map(|s| {
                s.into_iter().filter(|x| x.queue_name == "student").count()
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
                    if stmt.queue_name == "student" {
                        let action_type = match stmt.verb.as_str() {
                            "SubmitProgress" => StudentAction::SubmitProgress,
                            "CompleteLesson" => StudentAction::CompleteLesson,
                            "SubmitQuiz" => StudentAction::SubmitQuiz,
                            "SyncBookmark" => StudentAction::SyncBookmark,
                            _ => continue,
                        };

                        count += 1;
                        
                        let created_at = chrono::DateTime::parse_from_rfc3339(&stmt.timestamp)
                            .map(|dt| dt.timestamp())
                            .unwrap_or(0);

                        queue.push_back(QueueAction {
                            id: stmt.id,
                            action_type,
                            payload: serde_json::json!({ "course_id": stmt.object }),
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

    pub fn is_empty(&self) -> bool {
        self.core.is_empty()
    }
}

thread_local! {
    pub static STUDENT_QUEUE: StudentQueue = StudentQueue::new();
}

#[cfg(target_arch = "wasm32")]
async fn mock_sync_api(statements: &[crate::shared::storage::XapiStatement]) -> Result<Vec<String>, String> {
    gloo::timers::future::TimeoutFuture::new(800).await;
    let confirmed_ids: Vec<String> = statements.iter().map(|s| s.id.clone()).collect();
    Ok(confirmed_ids)
}