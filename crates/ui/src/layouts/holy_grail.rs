// crates/ui/src/layouts/holy_grail.rs
use leptos::prelude::*;

#[component]
pub fn HolyGrailLayout<H, L, C, R, F>(
    header: H,
    left: L,
    center: C,
    right: R,
    footer: F,
    #[prop(into, default = "gap-4".to_string())] class: String,
) -> impl IntoView
where
    H: Fn() -> AnyView + 'static,
    L: Fn() -> AnyView + 'static,
    C: Fn() -> AnyView + 'static,
    R: Fn() -> AnyView + 'static,
    F: Fn() -> AnyView + 'static,
{
    view! {
        // Задаем флекс-контейнер на всю высоту, чтобы футер всегда прижимался к низу
        <div class=format!("flex flex-col min-h-screen {}", class)>
            <header class="w-full shrink-0">{header()}</header>

            // Основная трехколоночная секция
            <div class="flex-1 w-full grid grid-cols-1 md:grid-cols-[auto_1fr_auto]">
                <aside class="w-full md:w-60 shrink-0">{left()}</aside>
                <main class="w-full min-w-0">{center()}</main>
                <aside class="w-full md:w-60 shrink-0">{right()}</aside>
            </div>

            <footer class="w-full shrink-0 mt-auto">{footer()}</footer>
        </div>
    }
}
