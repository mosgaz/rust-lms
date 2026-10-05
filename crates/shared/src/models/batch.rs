// crates/shared/src/models/batch.rs
//! Модель потока (Batch) — группы студентов, проходящих курсы вместе.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::tenant::TenantId;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор потока.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct BatchId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl BatchId {
    /// Генерирует новый случайный идентификатор потока (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BatchId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Статус потока.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(type_name = "VARCHAR", rename_all = "lowercase"))]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Черновик (поток ещё не запущен).
    Draft,
    /// Активный (идёт обучение).
    Active,
    /// Архивный (поток завершён или отменён, доступен только для чтения).
    Archived,
    /// Завершённый (все студенты успешно прошли поток).
    Completed,
}

impl fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Active => write!(f, "active"),
            Self::Archived => write!(f, "archived"),
            Self::Completed => write!(f, "completed"),
        }
    }
}

/// DTO потока для обмена данными между слоями.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// Уникальный идентификатор потока.
    pub id: BatchId,
    /// Идентификатор тенанта (для RLS).
    pub tenant_id: TenantId,
    /// Название потока.
    pub title: String,
    /// Описание потока.
    pub description: Option<String>,
    /// Текущий статус потока.
    pub status: BatchStatus,
    /// Дата начала обучения.
    pub start_date: Option<DateTime<Utc>>,
    /// Дата окончания обучения.
    pub end_date: Option<DateTime<Utc>>,
    /// Дедлайн для зачисления в поток.
    pub enrollment_deadline: Option<DateTime<Utc>>,
}