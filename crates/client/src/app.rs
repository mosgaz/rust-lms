use crate::ui::Button;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        // Tailwind-классы для центрирования и проверки работы стилей
        <main class="flex flex-col items-center justify-center min-h-screen bg-gray-50 p-4 gap-6">
            <h1 class="text-3xl font-bold text-gray-800">
                Проверка стилей и компонентов
            </h1>

            <Button>
                "Тестовая кнопка"
            </Button>
        </main>
    }
}
