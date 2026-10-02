// crates/client/src/student/pages/dashboard.rs
use leptos::prelude::*;
use crate::shared::state::AppQueueState;
use crate::student::services::offline_queue::STUDENT_QUEUE;

#[component]
pub fn StudentDashboard() -> impl IntoView {
    let queue_state = use_context::<AppQueueState>().expect("AppQueueState should be provided");
    
    let online_read = queue_state.is_online.read_only();
    let q_size_read = queue_state.student_queue_size.read_only(); // <-- ИМЕННО СТУДЕНЧЕСКАЯ ОЧЕРЕДЬ

    view! {
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <h2 class="text-2xl font-bold text-gray-800">"Кабинет студента"</h2>
                
                <div class="flex items-center gap-4">
                    <div class="flex items-center gap-2">
                        <div class={move || if online_read.get() {
                            "w-3 h-3 rounded-full bg-green-500"
                        } else {
                            "w-3 h-3 rounded-full bg-red-500"
                        }}></div>
                        <span class="text-sm text-gray-600">
                            {move || if online_read.get() { "Онлайн" } else { "Оффлайн" }}
                        </span>
                    </div>
                    
                    <div class="text-sm text-gray-600">
                        "В очереди: " {move || q_size_read.get()}
                    </div>
                </div>
            </div>
            
            <div class="grid grid-cols-1 gap-6 sm:grid-cols-2">
                <div class="p-6 bg-white rounded-lg shadow-sm border">
                    <h3 class="text-lg font-medium text-gray-900">"Пройдено курсов"</h3>
                    <p class="mt-2 text-3xl font-bold text-green-600">7</p>
                    
                    <button
                        class="mt-4 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
                        on:click=move |_| {
                            let q_sig = queue_state.student_queue_size.write_only();
                            STUDENT_QUEUE.with(|queue| {
                                queue.enqueue_progress("rust-101", 85, q_sig);
                            });
                        }
                    >
                        "Сохранить прогресс"
                    </button>
                </div>
                
                <div class="p-6 bg-white rounded-lg shadow-sm border">
                    <h3 class="text-lg font-medium text-gray-900">"Сертификаты"</h3>
                    <p class="mt-2 text-3xl font-bold text-blue-600">3</p>
                </div>
            </div>
        </div>
    }
}