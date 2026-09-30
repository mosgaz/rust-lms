// crates/ui/src/layouts/vertical.rs
use leptos::prelude::*;

#[component]
pub fn VerticalLayout(
    /// Дополнительные CSS классы (например, для управления отступами: gap-4, gap-6)
    #[prop(into, default = "gap-4".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("flex flex-col {}", class)>
            {children()}
        </div>
    }
}
