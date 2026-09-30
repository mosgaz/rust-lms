// crates/ui/src/layouts/horizontal.rs
use leptos::prelude::*;

#[component]
pub fn HorizontalLayout(
    /// Дополнительные CSS классы (например, gap-2, items-center, justify-between)
    #[prop(into, default = "gap-2 items-center flex-wrap".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("flex flex-row {}", class)>
            {children()}
        </div>
    }
}
