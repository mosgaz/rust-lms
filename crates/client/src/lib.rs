// crates/client/src/lib.rs
// ==========================================
// 1. Объявление внутренних модулей контуров
// ==========================================
pub mod admin;
pub mod app;
pub mod auth;
pub mod shared;      // <-- Оставляем локальную папку shared
pub mod student;
pub mod website;

// ==========================================
// 2. Фасады на внутренние крейты воркспейса
// ==========================================
pub use rust_lms_icons as icons;
pub use rust_lms_ui as ui;

// Экспорт основного компонента для SSR и гидрации
pub use app::App;

// ==========================================
// 3. Точка входа для WASM (Hydration)
// ==========================================
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;

    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}