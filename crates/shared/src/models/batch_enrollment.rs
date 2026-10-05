// crates/shared/src/models/batch_enrollment.rs
//! Модель зачисления пользователя в поток (Batch Enrollment).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::batch::BatchId;
use super::user::UserId;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор зачисления в поток.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct BatchEnrollmentId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl BatchEnrollmentId {
    /// Генерирует новый случайный идентификатор зачисления (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BatchEnrollmentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BatchEnrollmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Роль участника в потоке.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(type_name = "VARCHAR", rename_all = "lowercase"))]
#[serde(rename_all = "snake_case")]
pub enum BatchRole {
    /// Студент (проходит обучение).
    Student,
    /// Инструктор (ведёт поток, оценивает, управляет контентом).
    Instructor,
    /// Тьютор (проверяет задания, отвечает на вопросы).
    Tutor,
    /// Наблюдатель (руководитель/HR, имеет доступ только на чтение).
    Observer,
}

impl fmt::Display for BatchRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Student => write!(f, "student"),
            Self::Instructor => write!(f, "instructor"),
            Self::Tutor => write!(f, "tutor"),
            Self::Observer => write!(f, "observer"),
        }
    }
}

/// Статус зачисления в поток.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(type_name = "VARCHAR", rename_all = "lowercase"))]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentStatus {
    /// Активное зачисление (обучение идёт).
    Active,
    /// Успешно завершено.
    Completed,
    /// Отчислен (soft delete, запись сохраняется для истории).
    Dropped,
}

impl fmt::Display for EnrollmentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Completed => write!(f, "completed"),
            Self::Dropped => write!(f, "dropped"),
        }
    }
}

/// DTO зачисления пользователя в поток.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEnrollment {
    /// Уникальный идентификатор зачисления.
    pub id: BatchEnrollmentId,
    /// Идентификатор потока.
    pub batch_id: BatchId,
    /// Идентификатор пользователя.
    pub user_id: UserId,
    /// Роль пользователя в потоке.
    pub role: BatchRole,
    /// Дата и время зачисления.
    pub enrolled_at: DateTime<Utc>,
    /// Дата и время завершения (если статус Completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Текущий статус зачисления.
    pub status: EnrollmentStatus,
}