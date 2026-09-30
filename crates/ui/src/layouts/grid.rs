// crates/ui/src/layouts/grid.rs
use leptos::prelude::*;

#[component]
pub fn GridLayout(
    /// Настройка колонок (по умолчанию авто-заполнение с минимальной шириной 250px)
    #[prop(into, default = "grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-4".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("grid {}", class)>
            {children()}
        </div>
    }
}
