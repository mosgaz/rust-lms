// crates/ui/src/layouts/three_column.rs
use leptos::prelude::*;

#[component]
pub fn ThreeColumnLayout<L, C, R>(
    /// Левая колонка (например, навигация)
    left: L,
    /// Центральная резиновая колонка (основной контент)
    center: C,
    /// Правая колонка (например, контекстные виджеты)
    right: R,
    /// Дополнительные классы (например, gap-4)
    #[prop(into, default = "gap-4".to_string())]
    class: String,
) -> impl IntoView
where
    L: Fn() -> AnyView + 'static,
    C: Fn() -> AnyView + 'static,
    R: Fn() -> AnyView + 'static,
{
    view! {
        // Адаптивная трехколоночная сетка: на десктопе боковые колонки по 240px
        <div class=format!("grid grid-cols-1 md:grid-cols-[240px_1fr_240px] {}", class)>
            <div class="w-full">{left()}</div>
            <div class="w-full min-w-0">{center()}</div>
            <div class="w-full">{right()}</div>
        </div>
    }
}
