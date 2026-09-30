// crates/ui/src/layouts/stack.rs
use leptos::prelude::*;

// 💡 Совет по верстке: Внутри этого макета дочерние элементы,
// которые должны «летать» поверх основы, вызываются со стандартными
// классами Tailwind absolute top-0 left-0 z-10 и т.д.

#[component]
pub fn StackLayout(
    /// Дополнительные классы для контейнера (по умолчанию изолирует контекст наложения)
    #[prop(into, default = "w-full h-full".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        // relative создает систему координат для абсолютного позиционирования детей по оси Z
        <div class=format!("relative isolation-auto {}", class)>
            {children()}
        </div>
    }
}
