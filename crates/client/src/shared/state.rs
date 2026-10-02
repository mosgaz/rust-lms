// crates/client/src/shared/state.rs
use leptos::prelude::*;

#[derive(Clone)]
pub struct AppQueueState {
    pub is_online: RwSignal<bool>,
    pub student_queue_size: RwSignal<usize>,
    pub admin_queue_size: RwSignal<usize>,
}