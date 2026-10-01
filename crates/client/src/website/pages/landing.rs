use leptos::prelude::*;

#[component]
pub fn Landing() -> impl IntoView {
    view! {
        <div class="max-w-6xl mx-auto px-4 py-20 text-center">
            <h1 class="text-6xl font-extrabold text-gray-900 mb-6">
                Добро пожаловать в <span class="text-blue-600">Rust LMS</span>
            </h1>
            <p class="text-xl text-gray-600 mb-8 max-w-2xl mx-auto">
                Высокопроизводительная система управления обучением нового поколения на Rust.
            </p>
            <a href="/login" class="inline-block px-8 py-3 text-lg font-semibold text-white bg-blue-600 rounded-lg hover:bg-blue-700 transition">
                Начать обучение
            </a>
        </div>
    }
}