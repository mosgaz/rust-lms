// crates/ui/src/layouts/aspect_ratio.rs
use leptos::prelude::*;

#[component]
pub fn AspectRatioLayout(
    /// Пропорции. В Tailwind v4 можно передавать нативные классы:
    /// aspect-video (16:9), aspect-square (1:1) или произвольные aspect-[4/3]
    #[prop(into, default = "aspect-video".to_string())]
    ratio: String,
    /// Дополнительные стили отображения контейнера
    #[prop(into, default = "w-full overflow-hidden".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("{} {}", ratio, class)>
            {children()}
        </div>
    }
}
