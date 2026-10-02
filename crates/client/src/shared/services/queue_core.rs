// crates/client/src/shared/services/queue_core.rs
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use crate::shared::storage::{get_storage, XapiStatement};

#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub storage_key: String,
    pub max_retries: u8,
    pub queue_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAction<T: Clone + Serialize> {
    pub id: String,
    pub action_type: T,
    pub payload: serde_json::Value,
    pub created_at: i64,
    pub retry_count: u8,
    pub priority: QueuePriority,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueuePriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Clone)]
pub struct QueueCore<T: Clone + Serialize + for<'de> Deserialize<'de> + 'static> {
    pub(crate) queue: Rc<RefCell<VecDeque<QueueAction<T>>>>,
    config: QueueConfig,
}

impl<T: Clone + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + 'static> QueueCore<T> {
    pub fn new(config: QueueConfig) -> Self {
        let core = Self {
            queue: Rc::new(RefCell::new(VecDeque::new())),
            config,
        };
        #[cfg(target_arch = "wasm32")]
        core.load_from_storage();
        core
    }

    pub fn enqueue(&self, action_type: T, payload: serde_json::Value, priority: QueuePriority) {
        let action = QueueAction {
            id: uuid::Uuid::new_v4().to_string(),
            action_type,
            payload,
            created_at: chrono::Utc::now().timestamp(),
            retry_count: 0,
            priority,
        };

        self.queue.borrow_mut().push_back(action);
        self.sort_by_priority();
        self.persist_to_storage();
        self.notify_service_worker();
    }

    pub fn dequeue(&self) -> Option<QueueAction<T>> {
        let action = self.queue.borrow_mut().pop_front();
        if action.is_some() {
            self.persist_to_storage();
        }
        action
    }

    pub fn requeue(&self, mut action: QueueAction<T>) -> bool {
        action.retry_count += 1;
        if action.retry_count > self.config.max_retries {
            leptos::logging::warn!(
                "[{}] Action {} exceeded max retries ({})",
                self.config.queue_name,
                action.id,
                self.config.max_retries
            );
            return false;
        }
        self.queue.borrow_mut().push_back(action);
        self.persist_to_storage();
        true
    }

    fn sort_by_priority(&self) {
        let mut queue = self.queue.borrow_mut();
        let mut items: Vec<_> = queue.drain(..).collect();
        items.sort_by(|a, b| b.priority.cmp(&a.priority));
        *queue = items.into();
    }

    #[cfg(target_arch = "wasm32")]
    fn persist_to_storage(&self) {
        let queue_data: Vec<_> = self.queue.borrow().iter().cloned().collect();
        let queue_name = self.config.queue_name.clone();
        
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(storage) = get_storage().await {
                // §5.1: Получаем дельту времени (если ее нет, будет 0)
                let delta_ms = storage.get_client_clock().await
                    .ok()
                    .flatten()
                    .map(|c| c.delta_ms)
                    .unwrap_or(0);

                if delta_ms == 0 {
                    leptos::logging::warn!("[PWA-SYNC] Clock delta not measured. Using device time (risk of timestamp skew).");
                }

                for action in queue_data {
                    // §5.1: Корректируем время устройства на дельту
                    let adjusted_time_ms = (action.created_at * 1000) + delta_ms;
                    
                    // Форматируем в ISO 8601 (требование xAPI)
                    let timestamp_iso = chrono::DateTime::from_timestamp_millis(adjusted_time_ms)
                        .unwrap_or_else(|| chrono::Utc::now())
                        .to_rfc3339();

                    // Извлекаем object из payload (course_id или file_id)
                    let object = action.payload.get("course_id")
                        .or_else(|| action.payload.get("file_id"))
                        .or_else(|| action.payload.get("user_ids"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let statement = XapiStatement {
                        id: action.id,
                        timestamp: timestamp_iso,
                        stored_at: None, // §2.2: инициализируется как null
                        actor: "student_123".to_string(), // Заглушка: в реальности из контекста аутентификации
                        verb: format!("{:?}", action.action_type),
                        object,
                        queue_name: queue_name.clone(),
                        retry_count: action.retry_count,
                    };
                    let _ = storage.save_statement(&statement).await;
                }
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn load_from_storage(&self) {
        // Заглушка, реальное восстановление делается в специфичных для контура очередях
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn persist_to_storage(&self) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn load_from_storage(&self) {}

    #[cfg(target_arch = "wasm32")]
    fn notify_service_worker(&self) {
        use wasm_bindgen::JsValue;
        use web_sys::window;
        if let Some(window) = window() {
            if let Some(controller) = window.navigator().service_worker().controller() {
                let msg = JsValue::from_str(&format!(
                    r#"{{"type":"QUEUE_UPDATE","queue":"{}"}}"#,
                    self.config.queue_name
                ));
                let _ = controller.post_message(&msg);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn notify_service_worker(&self) {}

    pub fn len(&self) -> usize {
        self.queue.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}