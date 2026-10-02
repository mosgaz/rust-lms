// crates/client/src/student/services/offline_queue.rs
use crate::shared::services::queue_core::{QueueConfig, QueueCore, QueuePriority};
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
        use gloo::timers::future::TimeoutFuture;
        while let Some(action) = self.core.dequeue() {
            leptos::logging::log!(
                "[StudentQueue] Processing action: {:?} (id: {})",
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

    pub fn is_empty(&self) -> bool {
        self.core.is_empty()
    }
}

thread_local! {
    pub static STUDENT_QUEUE: StudentQueue = StudentQueue::new();
}