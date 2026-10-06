// crates/api/src/services/progress.rs
//! Сервисный слой для управления прогрессом обучения.
//!
//! Координирует работу репозиториев и инкапсулирует бизнес-логику,
//! включая генерацию событий для будущих интеграций (LRS/xAPI, Этап 13).

use rust_lms_shared::{LessonStatus, NodeId, ProgressUpdatedEvent, TenantId, UserId};

use crate::database::{
    LessonProgressRepository, LessonProgressRepositoryError, LessonProgressUpdateResult,
};

/// Сервис прогресса обучения.
#[derive(Debug, Clone)]
pub struct ProgressService {
    lesson_progress_repo: LessonProgressRepository,
}

impl ProgressService {
    /// Создает новый экземпляр сервиса.
    #[must_use]
    pub fn new(lesson_progress_repo: LessonProgressRepository) -> Self {
        Self {
            lesson_progress_repo,
        }
    }

    /// Обновляет прогресс урока, пересчитывает курс и генерирует событие.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_lesson_progress(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        node_id: NodeId,
        status: Option<LessonStatus>,
        score: Option<f64>,
        time_spent_seconds: Option<i32>,
        last_position: Option<i32>,
        client_modified_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<LessonProgressUpdateResult, LessonProgressRepositoryError> {
        // 1. Делегируем основную работу репозиторию (ACID, блокировки, пересчёт)
        let result = self
            .lesson_progress_repo
            .upsert_and_recalculate(
                tenant_id,
                user_id,
                node_id,
                status,
                score,
                time_spent_seconds,
                last_position,
                client_modified_at,
            )
            .await?;

        // 2. Генерация события для асинхронной обработки (Заготовка для Этапа 13: LRS xAPI)
        // TODO(Stage 13): Заменить на реальную публикацию в шину событий (e.g., event_bus.publish(event).await)
        let _event = ProgressUpdatedEvent::new(
            tenant_id,
            user_id,
            node_id,
            result.course_id,
            result.lesson_progress.status,
            result.lesson_progress.score,
            result.completion_triggered,
        );

        tracing::debug!(
            user_id = %user_id,
            course_id = %result.course_id,
            node_id = %node_id,
            completion_triggered = result.completion_triggered,
            "ProgressUpdatedEvent generated (stub for Stage 13)"
        );

        Ok(result)
    }
}