use leptos::prelude::*;

#[component]
pub fn StudentDashboard() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <h2 class="text-2xl font-bold text-gray-800">Мой прогресс</h2>
            
            <div class="grid grid-cols-1 gap-6 sm:grid-cols-2">
                <div class="p-6 bg-white rounded-lg shadow-sm border">
                    <h3 class="text-lg font-medium text-gray-900">Пройдено курсов</h3>
                    <p class="mt-2 text-3xl font-bold text-green-600">7</p>
                </div>
                <div class="p-6 bg-white rounded-lg shadow-sm border">
                    <h3 class="text-lg font-medium text-gray-900">Сертификаты</h3>
                    <p class="mt-2 text-3xl font-bold text-blue-600">3</p>
                </div>
            </div>
        </div>
    }
}