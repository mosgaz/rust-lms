//! Модель ответа (Answer) на вопрос в попытке.
//!
//! Каждый ответ связан с конкретной попыткой и вопросом.
//! Содержит текст ответа, правильность и заработанные баллы.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::{AttemptId, QuestionId, TenantId};

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор ответа.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct AnswerId(pub Uuid);

impl fmt::Display for AnswerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AnswerId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Модель ответа студента на вопрос.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    /// Уникальный идентификатор ответа.
    pub id: AnswerId,
    /// Идентификатор тенанта (для RLS).
    pub tenant_id: TenantId,
    /// Идентификатор попытки.
    pub attempt_id: AttemptId,
    /// Идентификатор вопроса.
    pub question_id: QuestionId,
    /// Текст ответа студента (или выбранные варианты для MultipleChoice).
    pub answer_text: String,
    /// Правильный ли ответ. NULL если не проверен.
    pub is_correct: Option<bool>,
    /// Заработанные баллы.
    pub points_earned: i32,
    /// Когда ответ дан.
    pub answered_at: chrono::DateTime<chrono::Utc>,
    /// Объяснение правильного ответа (если is_correct = false).
    pub explanation: Option<String>,
}

/// DTO для создания ответа.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnswerRequest {
    /// Идентификатор вопроса.
    pub question_id: QuestionId,
    /// Текст ответа (или JSON для MultipleChoice).
    pub answer_text: String,
}

/// DTO для ответа с деталями ответа.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerResponse {
    /// Ответ студента.
    pub answer: Answer,
    /// Вопрос, на который дан ответ.
    pub question: super::Question,
    /// Правильный ответ (для отображения после завершения попытки).
    pub correct_answer: Option<String>,
}