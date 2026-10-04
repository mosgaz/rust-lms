// crates/client/src/admin/pages/dashboard.rs
use leptos::prelude::*;
use crate::shared::state::AppQueueState;
use crate::admin::services::offline_queue::ADMIN_QUEUE; // <-- ИМПОРТ ОЧЕРЕДИ

#[component]
pub fn AdminDashboard() -> impl IntoView {
    let queue_state = use_context::<AppQueueState>().expect("AppQueueState should be provided");
    
    let online_read = queue_state.is_online.read_only();
    let q_size_read = queue_state.admin_queue_size.read_only();

    view! {
        <div class="space-y-6">
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                
				<h2 class="text-2xl font-bold text-gray-800">"Админ-дашборд"</h2>
                
                // <div class="flex items-center gap-4 bg-white px-4 py-2.5 rounded-lg shadow-sm border border-gray-200">
                //     <div class="flex items-center gap-2">
                //         <div class={move || if online_read.get() {
                //             "w-3 h-3 rounded-full bg-green-500"
                //         } else {
                //             "w-3 h-3 rounded-full bg-red-500 animate-pulse"
                //         }}></div>
                //         <span class="text-sm font-medium text-gray-700">
                //             {move || if online_read.get() { "Сеть: Онлайн" } else { "Сеть: Оффлайн" }}
                //         </span>
                //     </div>
                    
                //     <div class="w-px h-4 bg-gray-300"></div>
                    
                //     <div class="text-sm text-gray-600">
                //         "Действий в очереди: " 
                //         <span class="font-bold text-gray-900">{move || q_size_read.get()}</span>
                //     </div>
                // </div>

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
            
            <div class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
                <div class="p-6 bg-white rounded-lg shadow-sm border border-gray-200">
                    <h3 class="text-lg font-medium text-gray-900">"Пользователи"</h3>
                    <p class="mt-2 text-3xl font-bold text-blue-600">"1,248"</p>
                </div>
                <div class="p-6 bg-white rounded-lg shadow-sm border border-gray-200">
                    <h3 class="text-lg font-medium text-gray-900">"Курсы"</h3>
                    <p class="mt-2 text-3xl font-bold text-green-600">"42"</p>
                </div>
                <div class="p-6 bg-white rounded-lg shadow-sm border border-gray-200">
                    <h3 class="text-lg font-medium text-gray-900">"Лицензии"</h3>
                    <p class="mt-2 text-3xl font-bold text-purple-600">"Active"</p>
                </div>
            </div>

            <div class="p-6 bg-amber-50 rounded-lg shadow-sm border border-amber-200">
                <h3 class="text-lg font-medium text-amber-900 mb-2">"🛠 Тестирование оффлайн-очереди"</h3>
                <p class="text-sm text-amber-700 mb-4">
                    "Нажмите кнопку ниже, чтобы добавить действие в очередь. Если сеть отключена, действие накопится и выполнится автоматически при восстановлении соединения."
                </p>
                <button
                    class="px-4 py-2 bg-amber-600 text-white text-sm font-medium rounded-md hover:bg-amber-700 transition shadow-sm active:scale-[0.98]"
                    on:click=move |_| {
                        let q_sig = queue_state.admin_queue_size.write_only();
                        ADMIN_QUEUE.with(|queue| {
                            queue.enqueue_draft_announcement(
                                "Технические работы запланированы на 02:00".to_string(),
                                "all_users".to_string(),
                                q_sig
                            );
                        });
                        leptos::logging::log!("[Admin] Test action enqueued!");
                    }
                >
                    "Создать черновик объявления (Добавить в очередь)"
                </button>
            </div>
        </div>
    }
}