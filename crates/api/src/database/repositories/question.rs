// crates/api/src/database/repositories/question.rs
//! Репозиторий вопросов (Question) для тестов и оценок.
//!
//! Обеспечивает CRUD-операции и управление порядком вопросов в рамках курса.
//! Все операции учитывают RLS-изоляцию через `tenant_id`.

use chrono::{DateTime, Utc};
use rust_lms_shared::{
    AnswerOption, CourseId, CreateQuestionRequest, Question, QuestionId, QuestionType, TenantId,
    UpdateQuestionRequest,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Ошибки репозитория вопросов.
#[derive(Debug, thiserror::Error)]
pub enum QuestionRepositoryError {
    /// Ошибка базы данных.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    /// Вопрос не найден.
    #[error("Question not found: {0}")]
    NotFound(QuestionId),
    /// Дубликат порядка вопроса в рамках курса.
	#[error("Duplicate question order {order} in course {course_id}")]
	DuplicateOrder {
		/// Идентификатор курса, в котором возник конфликт.
		course_id: CourseId,
		/// Конфликтующий порядок вопроса.
		order: i32,
	},
    /// Неверный тип вопроса в базе данных.
    #[error("Invalid question type in database: {0}")]
    InvalidType(String),
    /// Ошибка сериализации вариантов ответа.
    #[error("Failed to serialize answer options: {0}")]
    Serialization(String),
}

/// Репозиторий вопросов.
#[derive(Debug, Clone)]
pub struct QuestionRepository {
    pool: PgPool,
}

impl QuestionRepository {
    /// Создаёт новый экземпляр репозитория.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создаёт новый вопрос в рамках курса.
    ///
    /// # Ошибки
    ///
    /// Возвращает `DuplicateOrder`, если вопрос с таким `order` уже существует в курсе.
    pub async fn create(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
        req: &CreateQuestionRequest,
    ) -> Result<Question, QuestionRepositoryError> {
        let id = QuestionId(Uuid::new_v4());

        // Сериализуем варианты ответа в JSONB
        let options_json: Option<serde_json::Value> = req
            .options
            .as_ref()
            .map(|opts| serde_json::to_value(opts))
            .transpose()
            .map_err(|e| QuestionRepositoryError::Serialization(e.to_string()))?;

        let result = sqlx::query(
            r#"
            INSERT INTO questions (
                id, tenant_id, course_id, title, description, question_type,
                options, correct_answer, points, "order"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (course_id, "order") DO NOTHING
            "#,
        )
        .bind(id.0)
        .bind(tenant_id.0)
        .bind(course_id.0)
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.question_type.to_string())
        .bind(&options_json)
        .bind(&req.correct_answer)
        .bind(req.points)
        .bind(req.order)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(QuestionRepositoryError::DuplicateOrder {
                course_id,
                order: req.order,
            });
        }

        self.get_by_id(tenant_id, id).await
    }

    /// Получает вопрос по идентификатору.
    ///
    /// # Ошибки
    ///
    /// Возвращает `NotFound`, если вопрос не существует или не принадлежит тенанту.
    pub async fn get_by_id(
        &self,
        tenant_id: TenantId,
        question_id: QuestionId,
    ) -> Result<Question, QuestionRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT
                id, tenant_id, course_id, title, description, question_type,
                options, correct_answer, points, "order", created_at, updated_at
            FROM questions
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(question_id.0)
        .bind(tenant_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(QuestionRepositoryError::NotFound(question_id))?;

        Self::row_to_question(&row)
    }

    /// Обновляет существующий вопрос.
    ///
    /// # Ошибки
    ///
    /// Возвращает `NotFound`, если вопрос не существует.
    /// Возвращает `DuplicateOrder`, если новый `order` конфликтует с существующим.
    pub async fn update(
        &self,
        tenant_id: TenantId,
        question_id: QuestionId,
        req: &UpdateQuestionRequest,
    ) -> Result<Question, QuestionRepositoryError> {
        // Получаем текущий вопрос для частичного обновления
        let current = self.get_by_id(tenant_id, question_id).await?;

        let title = req.title.clone().unwrap_or(current.title.clone());
        let description = req.description.clone().flatten().or(current.description.clone());
        let question_type = req.question_type.unwrap_or(current.question_type);
        let options = req.options.clone().flatten().or(current.options.clone());
        let correct_answer = req.correct_answer.clone().flatten().or(current.correct_answer.clone());
        let points = req.points.unwrap_or(current.points);
        let order = req.order.unwrap_or(current.order);

        // Сериализуем варианты ответа
        let options_json: Option<serde_json::Value> = options
            .as_ref()
            .map(|opts| serde_json::to_value(opts))
            .transpose()
            .map_err(|e| QuestionRepositoryError::Serialization(e.to_string()))?;

        // Проверяем конфликт порядка (если order изменился)
        if order != current.order {
            let conflict = sqlx::query(
                r#"
                SELECT COUNT(*) as cnt
                FROM questions
                WHERE course_id = $1 AND "order" = $2 AND id != $3 AND tenant_id = $4
                "#,
            )
            .bind(current.course_id.0)
            .bind(order)
            .bind(question_id.0)
            .bind(tenant_id.0)
            .fetch_one(&self.pool)
            .await?;

            let count: i64 = conflict.get("cnt");
            if count > 0 {
                return Err(QuestionRepositoryError::DuplicateOrder {
                    course_id: current.course_id,
                    order,
                });
            }
        }

        let result = sqlx::query(
            r#"
            UPDATE questions
            SET
                title = $1,
                description = $2,
                question_type = $3,
                options = $4,
                correct_answer = $5,
                points = $6,
                "order" = $7
            WHERE id = $8 AND tenant_id = $9
            "#,
        )
        .bind(&title)
        .bind(&description)
        .bind(question_type.to_string())
        .bind(&options_json)
        .bind(&correct_answer)
        .bind(points)
        .bind(order)
        .bind(question_id.0)
        .bind(tenant_id.0)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(QuestionRepositoryError::NotFound(question_id));
        }

        self.get_by_id(tenant_id, question_id).await
    }

    /// Удаляет вопрос.
    ///
    /// # Ошибки
    ///
    /// Возвращает `NotFound`, если вопрос не существует.
    pub async fn delete(
        &self,
        tenant_id: TenantId,
        question_id: QuestionId,
    ) -> Result<(), QuestionRepositoryError> {
        let result = sqlx::query(
            r#"DELETE FROM questions WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(question_id.0)
        .bind(tenant_id.0)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(QuestionRepositoryError::NotFound(question_id));
        }

        Ok(())
    }

    /// Возвращает список вопросов курса, отсортированных по порядку.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку базы данных при сбое запроса.
    pub async fn list_by_course(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
    ) -> Result<Vec<Question>, QuestionRepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, tenant_id, course_id, title, description, question_type,
                options, correct_answer, points, "order", created_at, updated_at
            FROM questions
            WHERE course_id = $1 AND tenant_id = $2
            ORDER BY "order" ASC
            "#,
        )
        .bind(course_id.0)
        .bind(tenant_id.0)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(Self::row_to_question).collect()
    }

    /// Изменяет порядок вопросов в курсе.
    ///
    /// Принимает список `(question_id, new_order)` и обновляет порядок в одной транзакции.
    ///
    /// # Ошибки
    ///
    /// Возвращает `NotFound`, если вопрос не существует.
    /// Возвращает `DuplicateOrder`, если новый порядок конфликтует.
    pub async fn reorder(
        &self,
        tenant_id: TenantId,
        course_id: CourseId,
        orders: &[(QuestionId, i32)],
    ) -> Result<(), QuestionRepositoryError> {
        let mut tx = self.pool.begin().await?;

        // Проверяем дубликаты порядка в новых данных
        let mut seen_orders = std::collections::HashSet::new();
        for (_, order) in orders {
            if !seen_orders.insert(order) {
                return Err(QuestionRepositoryError::DuplicateOrder {
                    course_id,
                    order: *order,
                });
            }
        }

        for (question_id, new_order) in orders {
            let result = sqlx::query(
                r#"
                UPDATE questions
                SET "order" = $1
                WHERE id = $2 AND tenant_id = $3 AND course_id = $4
                "#,
            )
            .bind(new_order)
            .bind(question_id.0)
            .bind(tenant_id.0)
            .bind(course_id.0)
            .execute(&mut *tx)
            .await?;

            if result.rows_affected() == 0 {
                return Err(QuestionRepositoryError::NotFound(*question_id));
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Преобразует строку базы данных в модель `Question`.
    fn row_to_question(row: &sqlx::postgres::PgRow) -> Result<Question, QuestionRepositoryError> {
        let id: Uuid = row.get("id");
        let tenant_id: Uuid = row.get("tenant_id");
        let course_id: Uuid = row.get("course_id");
        let title: String = row.get("title");
        let description: Option<String> = row.get("description");
        let question_type_str: String = row.get("question_type");
        let options_json: Option<serde_json::Value> = row.get("options");
        let correct_answer: Option<String> = row.get("correct_answer");
        let points: i32 = row.get("points");
        let order: i32 = row.get("order");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");

        // Парсим тип вопроса
        let question_type = Self::parse_question_type(&question_type_str)?;

        // Десериализуем варианты ответа
        let options: Option<Vec<AnswerOption>> = options_json
            .map(|json| serde_json::from_value(json))
            .transpose()
            .map_err(|e| QuestionRepositoryError::Serialization(e.to_string()))?;

        Ok(Question {
            id: QuestionId(id),
            tenant_id: TenantId(tenant_id),
            course_id: CourseId(course_id),
            title,
            description,
            question_type,
            options,
            correct_answer,
            points,
            order,
            created_at,
            updated_at,
        })
    }

    /// Парсит строковое представление типа вопроса в `QuestionType`.
    fn parse_question_type(s: &str) -> Result<QuestionType, QuestionRepositoryError> {
        match s {
            "multiple_choice" => Ok(QuestionType::MultipleChoice),
            "true_false" => Ok(QuestionType::TrueFalse),
            "short_answer" => Ok(QuestionType::ShortAnswer),
            "long_answer" => Ok(QuestionType::LongAnswer),
            _ => Err(QuestionRepositoryError::InvalidType(s.to_string())),
        }
    }
}