// crates/ui/src/lib.rs

// Привязываем внешние крейты к корню для бесперебойной работы макросов
pub extern crate leptos_router;
pub extern crate paste;
pub extern crate tw_merge;

// Объявление внутренних модулей библиотеки
pub mod components;
pub mod hooks;
pub mod layouts;
pub mod utils;

// Реэкспорт геометрических макетов (Layouts)
pub use layouts::absolute::AbsoluteLayout;
pub use layouts::aspect_ratio::AspectRatioLayout;
pub use layouts::centered::CenteredLayout;
pub use layouts::container::ContainerLayout;
pub use layouts::grid::GridLayout;
pub use layouts::holy_grail::HolyGrailLayout;
pub use layouts::horizontal::HorizontalLayout;
pub use layouts::masonry::MasonryLayout;
pub use layouts::spacer::SpacerLayout;
pub use layouts::split::SplitLayout;
pub use layouts::stack::StackLayout;
pub use layouts::sticky::StickyLayout;
pub use layouts::three_column::ThreeColumnLayout;
pub use layouts::two_column::TwoColumnLayout;
pub use layouts::vertical::VerticalLayout;
pub use layouts::viewport::ViewportLayout;
pub use layouts::wrap::WrapLayout;

// Реэкспорт интерактивных UI-компонентов (Components)
pub use components::accordion::*;
pub use components::action_bar::*;
pub use components::alert_dialog::*;
pub use components::avatar::*;
pub use components::animate::*;
pub use components::aspect_ratio::*;
pub use components::attachment::*;
pub use components::badge::*;
pub use components::breadcrumbs::*;
pub use components::button::*;
pub use components::card::*;
pub use components::checkbox::*;
pub use components::chips::*;
pub use components::dialog::*;
pub use components::drawer::*;
pub use components::expandable::*;
pub use components::input::*;
pub use components::label::*;
pub use components::sheet::*;
pub use components::sidenav::*;
pub use components::skeleton::*;

pub use utils::Utils;

// ==========================================
// Фасады на внутренние крейты воркспейса
// ==========================================
pub use rust_lms_icons as icons;

pub mod prelude {

    pub use crate::components::accordion::*;
    pub use crate::components::action_bar::*;
    pub use crate::components::alert_dialog::*;
    pub use crate::components::alert::*;
    pub use crate::components::animate::*;
    pub use crate::components::aspect_ratio::*;
    pub use crate::components::attachment::*;
    pub use crate::components::avatar::*;
    pub use crate::components::badge::*;
    pub use crate::components::breadcrumbs::*;
    pub use crate::components::button::*;
    pub use crate::components::card::*;
    pub use crate::components::checkbox::*;
    pub use crate::components::chips::*;
    pub use crate::components::dialog::*;
    pub use crate::components::drawer::*;
    pub use crate::components::expandable::*;
    pub use crate::components::input::*;
    pub use crate::components::label::*;
    pub use crate::components::sheet::*;
    pub use crate::components::sidenav::*;
    pub use crate::components::skeleton::*;

    pub use crate::layouts::*;
    pub use crate::utils::Utils;

}
