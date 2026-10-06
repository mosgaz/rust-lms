// crates/api/src/services/mod.rs
//! Сервисный слой приложения, инкапсулирующий бизнес-логику.

pub mod progress;
pub mod scoring;

pub use progress::ProgressService;
pub use scoring::{AttemptScoringResult, QuestionScoringResult, ScoringEngine, ScoringResult};