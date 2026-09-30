// crates/ui/src/layouts/spacer.rs
use leptos::prelude::*;

#[component]
pub fn SpacerLayout(
    /// Дополнительные классы для тонкой настройки (например, shrink-0, чтобы запретить сжатие)
    #[prop(into, default = "".to_string())]
    class: String,
) -> impl IntoView {
    view! {
        // flex-grow (или grow) заставляет элемент занять максимум свободного места
        <div class=format!("grow self-stretch {}", class) aria-hidden="true" />
    }
}
