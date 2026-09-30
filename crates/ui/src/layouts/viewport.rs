// crates/ui/src/layouts/viewport.rs
use leptos::prelude::*;

#[component]
pub fn ViewportLayout(
    /// Дополнительные классы (например, для смены фона всего приложения)
    #[prop(into, default = "".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        // h-dvh — динамическая высота экрана, overflow-hidden запрещает общий скролл страницы
        <div class=format!("w-screen h-dvh overflow-hidden flex flex-col {}", class)>
            {children()}
        </div>
    }
}
