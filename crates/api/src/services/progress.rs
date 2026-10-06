// crates/api/src/services/progress.rs
//! Сервисный слой для управления прогрессом обучения.
//!
//! Координирует работу репозиториев и инкапсулирует бизнес-логику,
//! включая генерацию событий для будущих интеграций (LRS/xAPI, Этап 13).

use rust_lms_shared::{
    CourseId, CourseProgressSummary, LessonProgress, LessonStatus, NodeId, ProgressUpdatedEvent,
    TenantId, UserId,
};

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

    /// Проверяет, является ли пользователь инструктором или администратором.
    pub async fn is_instructor_or_admin(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
    ) -> Result<bool, sqlx::Error> {
        self.lesson_progress_repo
            .is_instructor_or_admin(tenant_id, user_id)
            .await
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

    /// Получает детальный прогресс пользователя по конкретному курсу.
    ///
    /// # Errors
    ///
    /// Возвращает `LessonProgressRepositoryError::NotEnrolled`, если студент не зачислен.
    pub async fn get_user_course_progress(
        &self,
        tenant_id: TenantId,
        user_id: UserId,
        course_id: CourseId,
    ) -> Result<Vec<LessonProgress>, LessonProgressRepositoryError> {
        self.lesson_progress_repo
            .get_user_course_progress(tenant_id, user_id, course_id)
            .await
    }

    /// Получает прогресс всех студентов курса с пагинацией.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если:
    /// - Курс не найден
    /// - Ошибка базы данных
    pub async fn get_course_students_progress(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CourseProgressSummary>, LessonProgressRepositoryError> {
        self.lesson_progress_repo
            .get_course_students_progress(tenant_id, course_id, limit, offset)
            .await
            .map_err(LessonProgressRepositoryError::Database)
    }

    /// Принудительный пересчёт прогресса для всех зачисленных студентов курса.
    ///
    /// **Внимание:** Эта операция блокирует все зачисления курса на время выполнения.
    /// Для больших курсов (1000+ студентов) это может занять несколько секунд.
    pub async fn recalculate_course_progress(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<usize, LessonProgressRepositoryError> {
        self.lesson_progress_repo
            .recalculate_course_progress(tenant_id, course_id)
            .await
            .map_err(LessonProgressRepositoryError::Database)
    }
}