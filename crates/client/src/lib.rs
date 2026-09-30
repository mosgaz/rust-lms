pub mod app;

// ==========================================
// Фасады на внутренние крейты воркспейса
// ==========================================
pub use rust_lms_icons as icons;
pub use rust_lms_shared as shared;
pub use rust_lms_ui as ui;

// Экспорт основного компонента для SSR и гидрации
pub use app::App;

// ==========================================
// Точка входа для WASM (Hydration)
// ==========================================
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;

    // Перехват паник и вывод их в консоль браузера (крайне рекомендуется для отладки)
    console_error_panic_hook::set_once();

    // Монтируем приложение в <body>
    leptos::mount::hydrate_body(App);
}
