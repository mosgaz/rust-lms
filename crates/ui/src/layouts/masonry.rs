// crates/ui/src/layouts/masonry.rs
use leptos::prelude::*;

#[component]
pub fn MasonryLayout(
    /// Количество колонок на разных брейкпоинтах (по умолчанию от 1 до 3)
    #[prop(into, default = "columns-1 sm:columns-2 lg:columns-3 gap-4 space-y-4".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        // Внутри masonry важно, чтобы дочерние элементы имели класс break-inside-avoid
        <div class=format!("w-full {}", class)>
            {children()}
        </div>
    }
}
