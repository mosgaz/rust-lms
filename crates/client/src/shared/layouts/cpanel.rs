use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn CPanelLayout() -> impl IntoView {
    view! {
        <div class="flex min-h-screen bg-gray-100">
            <aside class="w-64 bg-slate-900 text-slate-300 flex flex-col">
                <div class="p-6 text-xl font-bold text-white border-b border-slate-800">
                    LMS Panel
                </div>
                <nav class="flex-1 p-4 space-y-2">
                    <a href="/admin" class="block px-4 py-2 rounded hover:bg-slate-800 hover:text-white">Админка</a>
                    <a href="/student" class="block px-4 py-2 rounded hover:bg-slate-800 hover:text-white">Студент</a>
                </nav>
                <div class="p-4 border-t border-slate-800">
                    <a href="/" class="text-sm hover:text-white">&larr; На сайт</a>
                </div>
            </aside>

            <div class="flex flex-col flex-1">
                <header class="flex items-center justify-between px-8 py-4 bg-white shadow-sm">
                    <h1 class="text-lg font-semibold text-gray-800">Панель управления</h1>
                    <div class="flex items-center space-x-4">
                        <span class="text-sm text-gray-600">user@rust-lms.local</span>
                        <div class="w-8 h-8 rounded-full bg-blue-500"></div>
                    </div>
                </header>

                <main class="flex-1 p-8 overflow-y-auto">
                    <Outlet />
                </main>
            </div>
        </div>
    }
}