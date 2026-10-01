use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn WebsiteLayout() -> impl IntoView {
    view! {
        <div class="flex flex-col min-h-screen bg-white">
            <nav class="flex items-center justify-between px-8 py-4 bg-white shadow-sm border-b">
                <a href="/" class="text-2xl font-bold text-blue-600">Rust LMS</a>
                <div class="space-x-6">
                    <a href="/" class="text-gray-600 hover:text-blue-600 transition">Главная</a>
                    <a href="/login" class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 transition">Войти</a>
                </div>
            </nav>
            <main class="flex-grow"><Outlet /></main>
            <footer class="px-8 py-6 text-center text-gray-500 bg-gray-50 border-t">
                <p>&copy; 2026 Rust LMS. Все права защищены.</p>
            </footer>
        </div>
    }
}