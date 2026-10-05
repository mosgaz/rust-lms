//! Модель прогресса студента по уроку (LessonProgress).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::Type;

use super::node::NodeId;
use super::tenant::TenantId;
use super::user::UserId;

/// Уникальный идентификатор записи прогресса урока.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct LessonProgressId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl LessonProgressId {
    /// Генерирует новый случайный идентификатор (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LessonProgressId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LessonProgressId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Статус прохождения урока.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(type_name = "VARCHAR"))]
#[serde(rename_all = "snake_case")]
pub enum LessonStatus {
    /// Урок ещё не начат.
    NotStarted,
    /// Урок в процессе прохождения.
    InProgress,
    /// Урок завершён.
    Completed,
}

impl LessonStatus {
    /// Возвращает строковое представление статуса (для SQL).
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

impl fmt::Display for LessonStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for LessonStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not_started" => Ok(Self::NotStarted),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            other => Err(format!("invalid lesson status: {other}")),
        }
    }
}

/// Прогресс студента по конкретному уроку.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonProgress {
    /// Уникальный идентификатор записи.
    pub id: LessonProgressId,
    /// Идентификатор тенанта (для RLS).
    pub tenant_id: TenantId,
    /// Идентификатор студента.
    pub user_id: UserId,
    /// Идентификатор урока (node_type = lesson).
    pub node_id: NodeId,
    /// Статус прохождения урока.
    pub status: LessonStatus,
    /// Балл за тест (0.0–1.0). NULL для нетестовых уроков.
    pub score: Option<f64>,
    /// Сдан ли тест. Вычисляется сервером на основе `score` и `passing_score`.
    pub passed: Option<bool>,
    /// Общее время в уроке (секунды).
    pub time_spent_seconds: i32,
    /// Количество попыток. Увеличивается только для тестов при `status = completed`.
    pub attempt_count: i32,
    /// Позиция в медиа (секунды) для возобновления.
    pub last_position: i32,
    /// Когда урок завершён (status = completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Время последнего изменения на клиенте (зарезервировано для Этапа 11).
    pub client_modified_at: Option<DateTime<Utc>>,
    /// Когда запись создана.
    pub created_at: DateTime<Utc>,
    /// Когда запись последний раз обновлена.
    pub updated_at: DateTime<Utc>,
}

/// DTO для входящего запроса обновления прогресса урока.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdateRequest {
    /// Идентификатор урока (обязательное поле).
    pub node_id: NodeId,
    /// Новый статус урока.
    pub status: Option<LessonStatus>,
    /// Новый балл за тест (0.0–1.0).
    pub score: Option<f64>,
    /// Новое время в уроке (секунды).
    pub time_spent_seconds: Option<i32>,
    /// Новая позиция в медиа (секунды).
    pub last_position: Option<i32>,
    /// Время изменения на клиенте (UTC, для офлайн-синхронизации).
    pub client_modified_at: Option<DateTime<Utc>>,
}

/// DTO для ответа клиенту после обновления прогресса.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressResponse {
    /// Обновлённый прогресс урока.
    pub lesson_progress: LessonProgress,
    /// Текущий прогресс курса (0.0–1.0).
    pub course_progress: f64,
    /// Статус курса (active/completed).
    pub course_status: String,
    /// Было ли инициировано завершение курса этим запросом.
    pub completion_triggered: bool,
}

/// Агрегированный прогресс студента по курсу.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseProgressSummary {
    /// Идентификатор курса.
    pub course_id: super::CourseId,
    /// Идентификатор студента.
    pub user_id: UserId,
    /// Общий прогресс курса (0.0–1.0).
    pub progress: f64,
    /// Статус курса (active/completed/dropped).
    pub status: String,
    /// Когда курс завершён (если status = completed).
    pub completed_at: Option<DateTime<Utc>>,
    /// Количество завершённых уроков.
    pub completed_lessons_count: i32,
    /// Общее количество уроков в курсе (не включая архивные).
    pub total_lessons_count: i32,
    /// Суммарный вес завершённых уроков.
    pub completed_lessons_weight: f64,
    /// Суммарный вес всех уроков курса.
    pub total_lessons_weight: f64,
}

/// Событие обновления прогресса (заготовка для Этапа 13 — LRS/xAPI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdatedEvent {
    /// Идентификатор тенанта.
    pub tenant_id: TenantId,
    /// Идентификатор студента.
    pub user_id: UserId,
    /// Идентификатор урока.
    pub node_id: NodeId,
    /// Идентификатор курса.
    pub course_id: super::CourseId,
    /// Новый статус урока.
    pub new_status: LessonStatus,
    /// Новый балл (если применимо).
    pub new_score: Option<f64>,
    /// Было ли инициировано завершение курса.
    pub completion_triggered: bool,
    /// Когда произошло событие.
    pub occurred_at: DateTime<Utc>,
}

impl ProgressUpdatedEvent {
    /// Создаёт новое событие обновления прогресса.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        user_id: UserId,
        node_id: NodeId,
        course_id: super::CourseId,
        new_status: LessonStatus,
        new_score: Option<f64>,
        completion_triggered: bool,
    ) -> Self {
        Self {
            tenant_id,
            user_id,
            node_id,
            course_id,
            new_status,
            new_score,
            completion_triggered,
            occurred_at: Utc::now(),
        }
    }
}