// crates/api/src/database/repositories/node.rs
//! Репозиторий для работы с узлами иерархии контента (Node).
//!
//! Использует паттерн Adjacency List + ltree для эффективных запросов поддеревьев.
//! Все операции tenant-scoped (RLS).

use rust_lms_shared::{CourseId, Node, NodeId, NodeType, TenantId};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};
use thiserror::Error;

use crate::database::rls::{RlsContext, RlsError};

/// Ошибки репозитория Node.
#[derive(Debug, Error)]
pub enum NodeRepositoryError {
    /// Ошибка базы данных.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    /// Узел не найден.
    #[error("node not found: {0}")]
    NotFound(NodeId),
    /// Курс не найден.
    #[error("course not found: {0}")]
    CourseNotFound(CourseId),
    /// Невалидный тип узла для данной операции.
    #[error("invalid node type: expected {expected}, got {actual}")]
    InvalidNodeType {
        /// Ожидаемый тип узла.
        expected: String,
        /// Фактический тип узла.
        actual: String,
    },
    /// Ошибка установки RLS-контекста.
    #[error("RLS context error: {0}")]
    RlsError(#[from] RlsError),
    /// Ошибка работы с ltree-путём.
    #[error("invalid ltree path: {0}")]
    InvalidPath(String),
}

/// Репозиторий для управления узлами иерархии контента.
#[derive(Clone)]
pub struct NodeRepository {
    pool: PgPool,
}

impl NodeRepository {
    /// Создаёт новый репозиторий Node.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт новый узел с автогенерацией ltree-пути.
    ///
    /// # Arguments
    /// * `tenant_id` — идентификатор тенанта.
    /// * `parent_id` — родительский узел (None для корневых).
    /// * `node_type` — тип узла.
    /// * `course_id` — связанный курс (для chapter/topic/lesson).
    /// * `title` — заголовок узла.
    /// * `title_i18n` — локализованные заголовки.
    /// * `description` — описание.
    /// * `metadata` — расширяемое содержимое.
    ///
    /// # Errors
    /// Возвращает ошибку, если родительский узел не найден или невалиден.
    pub async fn create(
        &self,
        tenant_id: TenantId,
        parent_id: Option<NodeId>,
        node_type: NodeType,
        course_id: Option<CourseId>,
        title: &str,
        title_i18n: Option<JsonValue>,
        description: Option<&str>,
        metadata: JsonValue,
    ) -> Result<Node, NodeRepositoryError> {
        let node_id = NodeId::new();

        tracing::info!(
            node_id = %node_id,
            parent_id = ?parent_id,
            node_type = %node_type,
            "Creating new node"
        );

        let mut tx = self.pool.begin().await?;
        let rls = RlsContext::new(tenant_id);
        rls.apply(&mut *tx).await?;

        // Вычисляем новый path
        let new_path = match parent_id {
            Some(pid) => {
                // Получаем path родителя
                let parent_path: String = sqlx::query_scalar(  // <-- String, не Option<String>
                    "SELECT path::text FROM nodes WHERE id = $1"
                )
                .bind(pid.0)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(NodeRepositoryError::NotFound(pid))?;

                format!("{}.{}", parent_path, node_id.0.to_string().replace('-', "_"))
            }
            None => node_id.0.to_string().replace('-', "_"),
        };

        // Вычисляем sort_order (максимальный среди братьев + 1)
        let sort_order: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM nodes WHERE parent_id IS NOT DISTINCT FROM $1"
        )
        .bind(parent_id.map(|p| p.0))
        .fetch_one(&mut *tx)
        .await?;

        let row: NodeRow = sqlx::query_as(
            r#"
            INSERT INTO nodes (id, tenant_id, parent_id, path, node_type, course_id, title, title_i18n, description, metadata, sort_order)
            VALUES ($1, $2, $3, $4::ltree, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            "#,
        )
        .bind(node_id.0)
        .bind(tenant_id.0)
        .bind(parent_id.map(|p| p.0))
        .bind(&new_path)
        .bind(node_type.to_string())
        .bind(course_id.map(|c| c.0))
        .bind(title)
        .bind(&title_i18n)
        .bind(description)
        .bind(&metadata)
        .bind(sort_order)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(self.row_to_node(row)?)
    }

    /// Получает узел по идентификатору.
    pub async fn find_by_id(
        &self,
        tenant_id: TenantId,
        node_id: NodeId,
    ) -> Result<Node, NodeRepositoryError> {
        tracing::debug!(node_id = %node_id, tenant_id = %tenant_id, "Fetching node by ID");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: NodeRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            FROM nodes WHERE id = $1
            "#,
        )
        .bind(node_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(NodeRepositoryError::NotFound(node_id))?;

        tx.commit().await?;
        Ok(self.row_to_node(row)?)
    }

    /// Получает прямых детей узла (отсортировано по sort_order).
    pub async fn find_children(
        &self,
        tenant_id: TenantId,
        parent_id: NodeId,
    ) -> Result<Vec<Node>, NodeRepositoryError> {
        tracing::debug!(parent_id = %parent_id, "Fetching children");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<NodeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            FROM nodes WHERE parent_id = $1 ORDER BY sort_order ASC
            "#,
        )
        .bind(parent_id.0)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        rows.into_iter().map(|r| self.row_to_node(r)).collect()
    }

    /// Получает всё поддерево узла через ltree-оператор `<@`.
    pub async fn find_subtree(
        &self,
        tenant_id: TenantId,
        node_id: NodeId,
    ) -> Result<Vec<Node>, NodeRepositoryError> {
        tracing::debug!(node_id = %node_id, "Fetching subtree via ltree");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        // Получаем path искомого узла
        let node_path: String = sqlx::query_scalar(
            "SELECT path::text FROM nodes WHERE id = $1"
        )
        .bind(node_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(NodeRepositoryError::NotFound(node_id))?;

        // Получаем всех потомков (включая сам узел)
        let rows: Vec<NodeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            FROM nodes WHERE path <@ $1::ltree ORDER BY path, sort_order ASC
            "#,
        )
        .bind(&node_path)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        rows.into_iter().map(|r| self.row_to_node(r)).collect()
    }

    /// Получает полное дерево курса (все узлы с course_id = курс).
    pub async fn find_course_tree(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<Vec<Node>, NodeRepositoryError> {
        tracing::debug!(course_id = %course_id, "Fetching course tree");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let rows: Vec<NodeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            FROM nodes WHERE course_id = $1 ORDER BY path, sort_order ASC
            "#,
        )
        .bind(course_id.0)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        rows.into_iter().map(|r| self.row_to_node(r)).collect()
    }

    /// Обновляет поля узла (title, description, metadata).
    pub async fn update(
        &self,
        tenant_id: TenantId,
        node_id: NodeId,
        title: Option<&str>,
        title_i18n: Option<JsonValue>,
        description: Option<&str>,
        metadata: Option<JsonValue>,
    ) -> Result<Node, NodeRepositoryError> {
        tracing::info!(node_id = %node_id, "Updating node");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: NodeRow = sqlx::query_as(
            r#"
            UPDATE nodes SET
                title = COALESCE($1, title),
                title_i18n = COALESCE($2, title_i18n),
                description = COALESCE($3, description),
                metadata = COALESCE($4, metadata),
                updated_at = NOW()
            WHERE id = $5
            RETURNING id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            "#,
        )
        .bind(title)
        .bind(&title_i18n)
        .bind(description)
        .bind(&metadata)
        .bind(node_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(NodeRepositoryError::NotFound(node_id))?;

        tx.commit().await?;
        Ok(self.row_to_node(row)?)
    }

    /// Перемещает узел под нового родителя (обновляет parent_id и path всего поддерева).
    pub async fn move_node(
        &self,
        tenant_id: TenantId,
        node_id: NodeId,
        new_parent_id: Option<NodeId>,
    ) -> Result<Node, NodeRepositoryError> {
        tracing::info!(node_id = %node_id, new_parent_id = ?new_parent_id, "Moving node");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        // Получаем текущий path узла
        let old_path: String = sqlx::query_scalar(
            "SELECT path::text FROM nodes WHERE id = $1"
        )
        .bind(node_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(NodeRepositoryError::NotFound(node_id))?;

        // Вычисляем новый path
        let new_path = match new_parent_id {
            Some(pid) => {
                let parent_path: String = sqlx::query_scalar(
                    "SELECT path::text FROM nodes WHERE id = $1"
                )
                .bind(pid.0)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(NodeRepositoryError::NotFound(pid))?;
                format!("{}.{}", parent_path, node_id.0.to_string().replace('-', "_"))
            }
            None => node_id.0.to_string().replace('-', "_"),
        };

        // Обновляем path для всего поддерева (заменяем old_path на new_path в начале)
        sqlx::query(
            r#"
            UPDATE nodes
            SET path = text2ltree($1 || substring(path::text from length($2) + 1)),
                parent_id = CASE WHEN id = $3 THEN $4 ELSE parent_id END,
                updated_at = NOW()
            WHERE path <@ $2::ltree
            "#,
        )
        .bind(&new_path)
        .bind(&old_path)
        .bind(node_id.0)
        .bind(new_parent_id.map(|p| p.0))
        .execute(&mut *tx)
        .await?;

        // Возвращаем обновлённый узел
        let row: NodeRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            FROM nodes WHERE id = $1
            "#,
        )
        .bind(node_id.0)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(self.row_to_node(row)?)
    }

    /// Удаляет узел (каскадно через FK ON DELETE CASCADE).
    pub async fn delete(
        &self,
        tenant_id: TenantId,
        node_id: NodeId,
    ) -> Result<(), NodeRepositoryError> {
        tracing::info!(node_id = %node_id, "Deleting node");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let result = sqlx::query("DELETE FROM nodes WHERE id = $1")
            .bind(node_id.0)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(NodeRepositoryError::NotFound(node_id));
        }

        tx.commit().await?;
        Ok(())
    }

    /// Изменяет порядок узла среди братьев.
    pub async fn reorder(
        &self,
        tenant_id: TenantId,
        node_id: NodeId,
        new_sort_order: i32,
    ) -> Result<Node, NodeRepositoryError> {
        tracing::info!(node_id = %node_id, new_sort_order = %new_sort_order, "Reordering node");

        let mut tx = self.pool.begin().await?;
        RlsContext::new(tenant_id).apply(&mut *tx).await?;

        let row: NodeRow = sqlx::query_as(
            r#"
            UPDATE nodes SET sort_order = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, parent_id, path::text, node_type, course_id, title, title_i18n, description, metadata, sort_order
            "#,
        )
        .bind(new_sort_order)
        .bind(node_id.0)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(NodeRepositoryError::NotFound(node_id))?;

        tx.commit().await?;
        Ok(self.row_to_node(row)?)
    }

    /// Вспомогательная функция для преобразования NodeRow в Node.
    fn row_to_node(&self, row: NodeRow) -> Result<Node, NodeRepositoryError> {
        let node_type = match row.node_type.as_str() {
            "program" => NodeType::Program,
            "course" => NodeType::Course,
            "chapter" => NodeType::Chapter,
            "topic" => NodeType::Topic,
            "lesson" => NodeType::Lesson,
            other => {
                return Err(NodeRepositoryError::InvalidNodeType {
                    expected: "program|course|chapter|topic|lesson".to_string(),
                    actual: other.to_string(),
                })
            }
        };

        Ok(Node {
            id: NodeId(row.id),
            tenant_id: TenantId(row.tenant_id),
            parent_id: row.parent_id.map(NodeId),
            node_type,
            course_id: row.course_id.map(CourseId),
            title: row.title,
            title_i18n: row.title_i18n,
            description: row.description,
            metadata: row.metadata,
            sort_order: row.sort_order,
        })
    }
}

/// Внутренняя структура для маппинга Node из БД.
#[derive(Debug, FromRow)]
struct NodeRow {
    id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    parent_id: Option<uuid::Uuid>,
    #[allow(dead_code)]
    path: String, // ltree как текст (не используется в DTO)
    node_type: String,
    course_id: Option<uuid::Uuid>,
    title: String,
    title_i18n: Option<JsonValue>,
    description: Option<String>,
    metadata: JsonValue,
    sort_order: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_found() {
        let node_id = NodeId::new();
        let err = NodeRepositoryError::NotFound(node_id);
        assert!(err.to_string().contains("node not found"));
    }

    #[test]
    fn test_error_display_invalid_node_type() {
        let err = NodeRepositoryError::InvalidNodeType {
            expected: "lesson".to_string(),
            actual: "invalid".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid node type"));
    }

    #[test]
    fn test_node_row_mapping() {
        let _row = NodeRow {
            id: uuid::Uuid::new_v4(),
            tenant_id: uuid::Uuid::new_v4(),
            parent_id: None,
            path: "abc123".to_string(),
            node_type: "lesson".to_string(),
            course_id: Some(uuid::Uuid::new_v4()),
            title: "Test Lesson".to_string(),
            title_i18n: None,
            description: Some("Description".to_string()),
            metadata: serde_json::json!({"content_type": "video"}),
            sort_order: 0,
        };
    }
}