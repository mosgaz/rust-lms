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
pub use components::accordion::{
    Accordion, AccordionContent, AccordionDescription, AccordionItem, AccordionTitle,
    AccordionTrigger,
};
pub use components::avatar::*;
pub use components::badge::*;
// pub use components::button::Button;
pub use components::button::*;

pub use utils::Utils;

// ==========================================
// Фасады на внутренние крейты воркспейса
// ==========================================
pub use rust_lms_icons as icons;

pub mod prelude {
    pub use crate::components::accordion::{
        Accordion, AccordionContent, AccordionDescription, AccordionItem, AccordionTitle,
        AccordionTrigger,
    };
    pub use crate::components::avatar::*;
    pub use crate::components::badge::*;
    pub use crate::components::button::*;
    pub use crate::layouts::*;
    pub use crate::utils::Utils;
}
