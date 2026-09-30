// crates/ui/src/layouts/centered.rs
use leptos::prelude::*;

#[component]
pub fn CenteredLayout(
    /// Минимальная высота (по умолчанию занимает всю доступную высоту родителя)
    #[prop(into, default = "min-h-full p-4".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("flex flex-col items-center justify-center {}", class)>
            {children()}
        </div>
    }
}
