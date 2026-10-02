// crates/client/src/admin/services/offline_queue.rs
use crate::shared::services::queue_core::{QueueCore, QueueConfig, QueuePriority};
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
        queue_size_signal.set(self.core.len()); // <-- Реактивное обновление UI
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn process_queue(&self, queue_size_signal: WriteSignal<usize>) {
        use gloo::timers::future::TimeoutFuture;
        while let Some(action) = self.core.dequeue() {
            leptos::logging::log!(
                "[AdminQueue] Processing action: {:?} (id: {})",
                action.action_type,
                action.id
            );
            TimeoutFuture::new(500).await;
        }
        queue_size_signal.set(self.core.len());
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn process_queue(&self, _queue_size_signal: WriteSignal<usize>) {}

    pub fn len(&self) -> usize {
        self.core.len()
    }
}

thread_local! {
    pub static ADMIN_QUEUE: AdminQueue = AdminQueue::new();
}