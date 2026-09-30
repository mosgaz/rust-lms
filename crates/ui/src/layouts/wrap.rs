// crates/ui/src/layouts/wrap.rs
use leptos::prelude::*;

#[component]
pub fn WrapLayout(
    /// Настройки выравнивания и отступов (по умолчанию gap-2 и центрирование по оси Y)
    #[prop(into, default = "gap-2 items-center".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        // flex-wrap — ключевое свойство для автопереноса строк
        <div class=format!("flex flex-row flex-wrap {}", class)>
            {children()}
        </div>
    }
}
