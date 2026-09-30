// crates/ui/src/layouts/absolute.rs
use leptos::prelude::*;

// 💡 Памятка по использованию: Дочерние элементы внутри AbsoluteLayout
// должны вызываться со стандартными Tailwind-классами абсолютного
// позиционирования (например, class="absolute top-4 left-10").

#[component]
pub fn AbsoluteLayout(
    /// Дополнительные классы для контейнера-холста
    #[prop(into, default = "w-full h-full".to_string())]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        // relative инициализирует точку отсчета (0,0) для абсолютно позиционированных детей
        <div class=format!("relative {}", class)>
            {children()}
        </div>
    }
}
