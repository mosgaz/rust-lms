// crates/shared/src/models/node.rs
//! Модель узла иерархии контента (Node).
//!
//! Node — это единая сущность для всех типов узлов в иерархии контента:
//! - Program (контейнер верхнего уровня)
//! - Course (основная единица учебного контента)
//! - Chapter (глава/раздел внутри курса)
//! - Topic (опциональный уровень между chapter и lesson)
//! - Lesson (атомарный элемент контента)
//!
//! Используется паттерн Adjacency List + ltree для эффективных запросов поддеревьев.
//! Содержимое узла (видео, текст, тест и т.д.) хранится в JSONB-поле `metadata`.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::tenant::TenantId;

#[cfg(feature = "server")]
use sqlx::Type;

/// Уникальный идентификатор узла иерархии контента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct NodeId(
    /// Внутренний UUID идентификатора.
    pub Uuid,
);

impl NodeId {
    /// Генерирует новый случайный идентификатор узла (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Тип узла иерархии контента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(type_name = "VARCHAR", rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// Контейнер верхнего уровня, объединяющий несколько курсов.
    Program,
    /// Основная единица учебного контента.
    Course,
    /// Глава/раздел внутри курса.
    Chapter,
    /// Опциональный уровень между chapter и lesson.
    Topic,
    /// Атомарный элемент контента (листовой узел).
    Lesson,
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Program => write!(f, "program"),
            Self::Course => write!(f, "course"),
            Self::Chapter => write!(f, "chapter"),
            Self::Topic => write!(f, "topic"),
            Self::Lesson => write!(f, "lesson"),
        }
    }
}

/// DTO узла иерархии контента для обмена данными между слоями.
///
/// Не содержит `path` (ltree) — это деталь реализации хранения,
/// которая используется только внутри `crates/api/src/database/repositories/node.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Уникальный идентификатор узла.
    pub id: NodeId,
    /// Идентификатор тенанта (для RLS).
    pub tenant_id: TenantId,
    /// Идентификатор родительского узла (None для корневых узлов: program, course без program, изолированный lesson).
    pub parent_id: Option<NodeId>,
    /// Тип узла (program/course/chapter/topic/lesson).
    pub node_type: NodeType,
    /// Идентификатор курса (для chapter/topic/lesson; None для program и изолированного lesson).
    pub course_id: Option<super::course::CourseId>,
    /// Заголовок узла (1-512 символов).
    pub title: String,
    /// Локализованные заголовки (BCP-47: {"ru": "...", "en": "..."}).
    pub title_i18n: Option<serde_json::Value>,
    /// Описание узла.
    pub description: Option<String>,
    /// Расширяемое содержимое узла (video, document, quiz, assignment и т.д.).
    pub metadata: serde_json::Value,
    /// Порядок сортировки среди братьев (детей одного родителя).
    pub sort_order: i32,
}