// crates/ui/src/layouts/split.rs
use leptos::prelude::*;

#[component]
pub fn SplitLayout<F, S>(
    /// Первый (левый или верхний) контейнер
    first: F,
    /// Второй (правый или нижний) контейнер
    second: S,
    /// Дополнительные классы для сетки (например, gap-4)
    #[prop(into, default = "gap-4".to_string())]
    class: String,
) -> impl IntoView
where
    F: Fn() -> AnyView + 'static,
    S: Fn() -> AnyView + 'static,
{
    view! {
        // На мобильных — в один столбец, на md и выше — две равные колонки (1fr 1fr)
        <div class=format!("grid grid-cols-1 md:grid-cols-2 {}", class)>
            <div class="w-full min-w-0">{first()}</div>
            <div class="w-full min-w-0">{second()}</div>
        </div>
    }
}
