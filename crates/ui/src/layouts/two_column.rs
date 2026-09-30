// crates/ui/src/layouts/two_column.rs
use leptos::prelude::*;

#[component]
pub fn TwoColumnLayout<L, R>(
    /// Слот для левой (или фиксированной) колонки
    left: L,
    /// Слот для правой (или гибкой) колонки
    right: R,
    /// Дополнительные классы для контейнера (например, для смены порядка на десктопе)
    #[prop(into, default = "gap-6".to_string())]
    class: String,
) -> impl IntoView
where
    L: Fn() -> AnyView + 'static,
    R: Fn() -> AnyView + 'static,
{
    view! {
        // На мобильных выстраивается в стек, на экранах md и выше — в сетку
        <div class=format!("grid grid-cols-1 md:grid-cols-[250px_1fr] {}", class)>
            // Левая фиксированная колонка (например, сайдбар)
            <div class="w-full">
                {left()}
            </div>
            // Правая резиновая колонка (основной контент)
            <div class="w-full min-w-0">
                {right()}
            </div>
        </div>
    }
}
