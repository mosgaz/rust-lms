// crates/ui/src/layouts/container.rs
use leptos::prelude::*;

#[component]
pub fn ContainerLayout(
    /// Максимальная ширина. В Tailwind v4 настраивается через max-w-5xl, max-w-7xl, max-w-screen-xl и т.д.
    #[prop(into, default = "max-w-7xl".to_string())]
    max_w: String,
    /// Дополнительные классы (по умолчанию задаются адаптивные безопасные паддинги по бокам)
    #[prop(into, default = "px-4 sm:px-6 lg:px-8".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        // mx-auto — центрирует блок по горизонтали внутри вьюпорта
        <div class=format!("w-full mx-auto {} {}", max_w, class)>
            {children()}
        </div>
    }
}
