// crates/ui/src/layouts/sticky.rs
use leptos::prelude::*;

#[component]
pub fn StickyLayout(
    /// Позиция прилипания. В Tailwind v4 настраивается через top-0, bottom-0, left-0, right-0
    #[prop(into, default = "top-0".to_string())]
    position: String,
    /// Дополнительные классы (по умолчанию включает z-индекс, чтобы контент не перекрывался при скролле)
    #[prop(into, default = "z-40".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        // sticky включает нативный механизм прилипания браузера
        <div class=format!("sticky {} {}", position, class)>
            {children()}
        </div>
    }
}
