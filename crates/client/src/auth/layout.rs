// crates/client/src/auth/layout.rs
// TODO: https://leptos.rust-ui.com/blocks/login
// TODO: https://leptos.rust-ui.com/view/login02
// TODO: Use crates/ui/layouts/split Layout
use leptos::prelude::*;

#[component]
pub fn AuthLayout(children: Children) -> impl IntoView {
    view! {
        <div class="flex items-center justify-center min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
            <div class="w-full max-w-md p-8 bg-white rounded-xl shadow-lg">
                // Здесь рендерится содержимое (Login, Register, Reset и т.д.)
                {children()}
            </div>
        </div>
    }
}