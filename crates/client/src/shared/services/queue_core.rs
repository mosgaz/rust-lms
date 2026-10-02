// crates/client/src/shared/services/queue_core.rs
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

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
pub struct QueueCore<T: Clone + Serialize + for<'de> Deserialize<'de>> {
    queue: Rc<RefCell<VecDeque<QueueAction<T>>>>,
    config: QueueConfig,
}

impl<T: Clone + Serialize + for<'de> Deserialize<'de>> QueueCore<T> {
    pub fn new(config: QueueConfig) -> Self {
        let core = Self {
            queue: Rc::new(RefCell::new(VecDeque::new())),
            config,
        };
        // Загружаем из хранилища ТОЛЬКО в браузере
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

    // --- WASM-специфичные методы ---
    #[cfg(target_arch = "wasm32")]
    fn persist_to_storage(&self) {
        use wasm_bindgen::JsValue;
        use web_sys::window;
        let queue_data: Vec<_> = self.queue.borrow().iter().cloned().collect();
        if let Ok(json) = serde_json::to_string(&queue_data) {
            if let Some(window) = window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item(&self.config.storage_key, &json);
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_from_storage(&self) {
        use web_sys::window;
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Some(json) = storage.get_item(&self.config.storage_key).ok().flatten() {
                    if let Ok(queue_data) = serde_json::from_str::<Vec<QueueAction<T>>>(&json) {
                        *self.queue.borrow_mut() = queue_data.into();
                    }
                }
            }
        }
    }

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

    // --- Серверные заглушки (no-op) ---
    #[cfg(not(target_arch = "wasm32"))]
    fn persist_to_storage(&self) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn load_from_storage(&self) {}

    #[cfg(not(target_arch = "wasm32"))]
    fn notify_service_worker(&self) {}

    // --- Общие методы ---
    pub fn len(&self) -> usize {
        self.queue.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}