// crates/shared/src/models/course.rs
//! Модель курса (Course) — основной единицы учебного контента.
//!
//! Course содержит главы (Chapter), которые, в свою очередь, содержат уроки (Lesson).
//! Курс может быть самостоятельным (self-paced) или проводиться через Потоки (Batch).
//!
//! Иерархия контента реализуется через таблицу `nodes` (Adjacency List + ltree),
//! где Course может быть как корневым узлом, так и частью Program.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::tenant::TenantId;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор курса.
///
/// Используется newtype-паттерн для типобезопасности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct CourseId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl CourseId {
    /// Генерирует новый случайный идентификатор курса (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CourseId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CourseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// DTO курса для обмена данными между слоями.
///
/// Содержит метаданные курса (версия, правила сертификации и т.д.).
/// Структура курса (дерево глав/тем/уроков) хранится в таблице `nodes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    /// Уникальный идентификатор курса.
    pub id: CourseId,
    /// Идентификатор тенанта (для RLS).
    pub tenant_id: TenantId,
    /// Заголовок курса (основной язык тенанта).
    pub title: String,
    /// Локализованные заголовки (BCP-47: {"ru": "...", "en": "..."}).
    pub title_i18n: Option<serde_json::Value>,
    /// Описание курса.
    pub description: Option<String>,
    /// Локализованные описания.
    pub description_i18n: Option<serde_json::Value>,
    /// Текущая версия структуры курса (монотонно растёт при публикации).
    pub version: i32,
    /// Правила автоматической выдачи сертификатов при завершении курса.
    pub certification_rules: Option<serde_json::Value>,
}