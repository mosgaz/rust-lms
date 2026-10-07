// crates/api/src/services/mod.rs
//! Сервисный слой приложения, инкапсулирующий бизнес-логику.

pub mod certificate;
pub mod email;
pub mod progress;
pub mod scoring;

pub use certificate::CertificateService;
pub use email::{EmailService, SmtpSettings};
pub use progress::ProgressService;
pub use scoring::{AttemptScoringResult, QuestionScoringResult, ScoringEngine, ScoringResult};