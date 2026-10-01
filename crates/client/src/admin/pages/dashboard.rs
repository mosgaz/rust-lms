use leptos::prelude::*;

#[component]
pub fn AdminDashboard() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <h2 class="text-2xl font-bold text-gray-800">Админ-дашборд</h2>
            
            <div class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
                <div class="p-6 bg-white rounded-lg shadow-sm border">
                    <h3 class="text-lg font-medium text-gray-900">Пользователи</h3>
                    <p class="mt-2 text-3xl font-bold text-blue-600">1,248</p>
                </div>
                <div class="p-6 bg-white rounded-lg shadow-sm border">
                    <h3 class="text-lg font-medium text-gray-900">Курсы</h3>
                    <p class="mt-2 text-3xl font-bold text-green-600">42</p>
                </div>
                <div class="p-6 bg-white rounded-lg shadow-sm border">
                    <h3 class="text-lg font-medium text-gray-900">Лицензии</h3>
                    <p class="mt-2 text-3xl font-bold text-purple-600">Active</p>
                </div>
            </div>
        </div>
    }
}