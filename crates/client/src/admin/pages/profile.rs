// crates/client/src/admin/pages/profile.rs
use leptos::prelude::*;

#[component]
pub fn AdminProfile() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <h1 class="text-2xl font-bold">"Профиль администратора"</h1>
            <p class="text-muted-foreground">
                "Тестовая страница для проверки breadcrumbs."
            </p>
        </div>
    }
}