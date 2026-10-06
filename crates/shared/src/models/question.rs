//! Модель вопроса (Question) для тестов и оценок.
//!
//! Вопрос может принадлежать тесту (Quiz) или быть самостоятельным.
//! Поддерживаются типы: множественный выбор, правда/ложь, короткий ответ, развёрнутый ответ.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::{CourseId, TenantId};

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор вопроса.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct QuestionId(pub Uuid);

impl fmt::Display for QuestionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for QuestionId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Тип вопроса.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(type_name = "VARCHAR"))]
pub enum QuestionType {
    /// Множественный выбор (один или несколько правильных ответов).
    MultipleChoice,
    /// Правда/ложь.
    TrueFalse,
    /// Короткий ответ (одно слово или фраза).
    ShortAnswer,
    /// Развёрнутый ответ (эссе).
    LongAnswer,
}

impl fmt::Display for QuestionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MultipleChoice => write!(f, "multiple_choice"),
            Self::TrueFalse => write!(f, "true_false"),
            Self::ShortAnswer => write!(f, "short_answer"),
            Self::LongAnswer => write!(f, "long_answer"),
        }
    }
}

/// Вариант ответа для вопросов типа MultipleChoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerOption {
    /// Текст варианта.
    pub text: String,
    /// Правильный ли это вариант.
    pub is_correct: bool,
}

/// Модель вопроса.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// Уникальный идентификатор вопроса.
    pub id: QuestionId,
    /// Идентификатор тенанта (для RLS).
    pub tenant_id: TenantId,
    /// Идентификатор курса, к которому принадлежит вопрос.
    pub course_id: CourseId,
    /// Заголовок или текст вопроса.
    pub title: String,
    /// Подробное описание вопроса (опционально).
    pub description: Option<String>,
    /// Тип вопроса.
    pub question_type: QuestionType,
    /// Варианты ответов (для MultipleChoice).
    pub options: Option<Vec<AnswerOption>>,
    /// Правильный ответ (для ShortAnswer/TrueFalse).
    pub correct_answer: Option<String>,
    /// Баллы за правильный ответ.
    pub points: i32,
    /// Порядок вопроса в тесте.
    pub order: i32,
    /// Когда вопрос создан.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Когда вопрос обновлён.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// DTO для создания вопроса.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuestionRequest {
    /// Заголовок вопроса.
    pub title: String,
    /// Описание вопроса.
    pub description: Option<String>,
    /// Тип вопроса.
    pub question_type: QuestionType,
    /// Варианты ответов (для MultipleChoice).
    pub options: Option<Vec<AnswerOption>>,
    /// Правильный ответ (для ShortAnswer/TrueFalse).
    pub correct_answer: Option<String>,
    /// Баллы за правильный ответ.
    pub points: i32,
    /// Порядок вопроса.
    pub order: i32,
}

/// DTO для обновления вопроса.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateQuestionRequest {
    /// Новый заголовок.
    pub title: Option<String>,
    /// Новое описание.
    pub description: Option<Option<String>>,
    /// Новый тип.
    pub question_type: Option<QuestionType>,
    /// Новые варианты ответов.
    pub options: Option<Option<Vec<AnswerOption>>>,
    /// Новый правильный ответ.
    pub correct_answer: Option<Option<String>>,
    /// Новые баллы.
    pub points: Option<i32>,
    /// Новый порядок.
    pub order: Option<i32>,
}