//! Модель попытки (Attempt) прохождения теста.
//!
//! Каждая попытка связана с пользователем и тестом (курсом).
//! Содержит статус, время начала/завершения и итоговый балл.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::{CourseId, TenantId, UserId};

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор попытки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct AttemptId(pub Uuid);

impl fmt::Display for AttemptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AttemptId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Статус попытки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(type_name = "VARCHAR"))]
pub enum AttemptStatus {
    /// В процессе прохождения.
    InProgress,
    /// Завершена.
    Completed,
    /// Просрочена (превышен лимит времени).
    TimedOut,
    /// Отменена пользователем.
    Abandoned,
}

impl fmt::Display for AttemptStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::TimedOut => write!(f, "timed_out"),
            Self::Abandoned => write!(f, "abandoned"),
        }
    }
}

/// Модель попытки прохождения теста.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    /// Уникальный идентификатор попытки.
    pub id: AttemptId,
    /// Идентификатор тенанта (для RLS).
    pub tenant_id: TenantId,
    /// Идентификатор студента.
    pub user_id: UserId,
    /// Идентификатор курса (тест привязан к курсу).
    pub course_id: CourseId,
    /// Статус попытки.
    pub status: AttemptStatus,
    /// Когда попытка начата.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Когда попытка завершена (если status = completed).
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Итоговый балл (0.0–1.0). NULL если не завершена.
    pub score: Option<f64>,
    /// Сдан ли тест. Вычисляется сервером.
    pub passed: Option<bool>,
    /// Лимит времени в секундах (NULL = без лимита).
    pub time_limit_seconds: Option<i32>,
    /// Фактически затраченное время (секунды).
    pub time_spent_seconds: i32,
    /// Количество попыток этого студента по этому тесту.
    pub attempt_number: i32,
}

/// DTO для создания новой попытки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttemptRequest {
    /// Идентификатор курса (теста).
    pub course_id: CourseId,
    /// Лимит времени в секундах (опционально).
    pub time_limit_seconds: Option<i32>,
}

/// DTO для ответа при создании попытки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptResponse {
    /// Созданная попытка.
    pub attempt: Attempt,
    /// Список вопросов для этой попытки.
    pub questions: Vec<super::Question>,
}

/// DTO для завершения попытки.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteAttemptRequest {
    /// Ответы студента.
    pub answers: Vec<super::CreateAnswerRequest>,
}