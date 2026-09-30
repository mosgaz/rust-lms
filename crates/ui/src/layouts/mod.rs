// crates/ui/src/layouts/mod.rs

// 1. Объявление всех геометрических подмодулей
pub mod absolute;
pub mod aspect_ratio;
pub mod centered;
pub mod container;
pub mod grid;
pub mod holy_grail;
pub mod horizontal;
pub mod masonry;
pub mod spacer;
pub mod split;
pub mod stack;
pub mod sticky;
pub mod three_column;
pub mod two_column;
pub mod vertical;
pub mod viewport;
pub mod wrap;

// 2. Публичный реэкспорт компонентов для удобного импорта
pub use absolute::AbsoluteLayout;
pub use aspect_ratio::AspectRatioLayout;
pub use centered::CenteredLayout;
pub use container::ContainerLayout;
pub use grid::GridLayout;
pub use holy_grail::HolyGrailLayout;
pub use horizontal::HorizontalLayout;
pub use masonry::MasonryLayout;
pub use spacer::SpacerLayout;
pub use split::SplitLayout;
pub use stack::StackLayout;
pub use sticky::StickyLayout;
pub use three_column::ThreeColumnLayout;
pub use two_column::TwoColumnLayout;
pub use vertical::VerticalLayout;
pub use viewport::ViewportLayout;
pub use wrap::WrapLayout;
