// crates/shared/src/models/course_enrollment.rs
//! Модель индивидуального зачисления пользователя на курс (Course Enrollment).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::batch_enrollment::EnrollmentStatus;
use super::course::CourseId;
use super::user::UserId;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор зачисления на курс.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct CourseEnrollmentId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl CourseEnrollmentId {
    /// Генерирует новый случайный идентификатор зачисления (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CourseEnrollmentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CourseEnrollmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// DTO индивидуального зачисления пользователя на курс.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseEnrollment {
    /// Уникальный идентификатор зачисления.
    pub id: CourseEnrollmentId,
    /// Идентификатор курса.
    pub course_id: CourseId,
    /// Идентификатор пользователя.
    pub user_id: UserId,
    /// Дата и время зачисления.
    pub enrolled_at: DateTime<Utc>,
    /// Дата и время завершения (если статус Completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Прогресс прохождения курса (0.0 – 1.0).
    pub progress: f64,
    /// Текущий статус зачисления.
    pub status: EnrollmentStatus,
}