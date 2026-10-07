// crates/api/src/http/handlers/assessment.rs
//! Обработчики для работы с тестами и оценками (Assessments Engine).

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_lms_shared::{CourseId, TenantId, UserId};
use rust_lms_shared::models::attempt::{AttemptId, CompleteAttemptRequest, CreateAttemptRequest};
use rust_lms_shared::models::lesson_progress::LessonStatus;
use rust_lms_shared::models::question::{CreateQuestionRequest, QuestionId};
use uuid::Uuid;

use crate::database::{AttemptRepositoryError, ScoredAnswer};

use super::{ApiResponse, AppState};

/// Создаёт новый вопрос для курса в контексте текущего тенанта.
pub async fn create_question(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
    Json(payload): Json<CreateQuestionRequest>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    tracing::info!(tenant_id = %tenant_id, course_id = %cid, "Creating new question");

    match state.question_repo.create(tenant_id, cid, &payload).await {
        Ok(question) => {
            let response = ApiResponse::ok(question);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create question");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Получает список вопросов для указанного курса.
pub async fn list_questions(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    match state.question_repo.list_by_course(tenant_id, cid).await {
        Ok(questions) => {
            let response = ApiResponse::ok(questions);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list questions");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Начинает новую попытку прохождения теста для пользователя.
pub async fn start_attempt(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Path(course_id): Path<Uuid>,
    Json(payload): Json<CreateAttemptRequest>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    tracing::info!(tenant_id = %tenant_id, user_id = %user_id, course_id = %cid, "Starting new attempt");

    match state
        .attempt_repo
        .create(tenant_id, user_id, cid, payload.time_limit_seconds)
        .await
    {
        Ok(response) => {
            let api_response = ApiResponse::ok(response);
            (StatusCode::CREATED, Json(api_response)).into_response()
        }
        Err(AttemptRepositoryError::MaxAttemptsReached { .. }) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Attempt limit exceeded");
            (StatusCode::CONFLICT, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to start attempt");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Сохраняет ответ на вопрос в рамках текущей попытки (инкрементальное сохранение).
pub async fn submit_answer(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Path(attempt_id): Path<Uuid>,
    Json(payload): Json<rust_lms_shared::models::answer::CreateAnswerRequest>,
) -> impl IntoResponse {
    let aid = AttemptId(attempt_id);
    tracing::info!(tenant_id = %tenant_id, user_id = %user_id, attempt_id = %aid, "Submitting answer");

    // 1. Получаем вопрос для скоринга
    let question = match state.question_repo.get_by_id(tenant_id, payload.question_id).await {
        Ok(q) => q,
        Err(e) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    // 2. Подсчитываем баллы за этот ответ
    let scoring_result = crate::services::ScoringEngine::new().score_answer(&question, &payload.answer_text);

    // 3. Формируем ScoredAnswer для сохранения
    let scored_answer = ScoredAnswer {
        question_id: payload.question_id,
        answer_text: payload.answer_text,
        is_correct: scoring_result.is_correct,
        points_earned: scoring_result.points_earned,
        explanation: scoring_result.explanation,
    };

    // 4. Сохраняем в БД
    match state.attempt_repo.save_answers(tenant_id, aid, &[scored_answer]).await {
        Ok(()) => {
            let response = ApiResponse::ok(serde_json::json!({
                "is_correct": scoring_result.is_correct,
                "points_earned": scoring_result.points_earned
            }));
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to save answer");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Завершает попытку, подсчитывает баллы и обновляет прогресс урока.
pub async fn complete_attempt(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Path((attempt_id, node_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CompleteAttemptRequest>,
) -> impl IntoResponse {
    let aid = AttemptId(attempt_id);
    let nid = rust_lms_shared::models::node::NodeId(node_id);
    tracing::info!(tenant_id = %tenant_id, user_id = %user_id, attempt_id = %aid, "Completing attempt");

    // 1. Получаем данные попытки
    let attempt = match state.attempt_repo.get_by_id(tenant_id, aid).await {
        Ok(a) => a,
        Err(_) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Attempt not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    // 2. Получаем вопросы курса для скоринга
    let questions = match state.question_repo.list_by_course(tenant_id, attempt.course_id).await {
        Ok(q) => q,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch questions for scoring");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 3. Преобразуем ответы в формат, ожидаемый ScoringEngine: &[(QuestionId, String)]
    let formatted_answers: Vec<(QuestionId, String)> = payload
        .answers
        .into_iter()
        .map(|ans| (ans.question_id, ans.answer_text.clone()))
        .collect();

    // 4. Вычисляем баллы через ScoringEngine (порог сдачи 0.7)
    let scoring_result = crate::services::ScoringEngine::new().score_attempt(&questions, &formatted_answers, 0.7);

    // 5. Формируем список ответов с результатами для сохранения в БД
    let scored_answers: Vec<ScoredAnswer> = scoring_result
        .results
        .iter()
        .map(|qr| {
            let answer_text = formatted_answers
                .iter()
                .find(|(qid, _)| *qid == qr.question_id)
                .map(|(_, text)| text.clone())
                .unwrap_or_default();

            ScoredAnswer {
                question_id: qr.question_id,
                answer_text,
                is_correct: qr.scoring.is_correct,
                points_earned: qr.scoring.points_earned,
                explanation: qr.scoring.explanation.clone(),
            }
        })
        .collect();

    // 6. Сохраняем ответы в БД
    if let Err(e) = state.attempt_repo.save_answers(tenant_id, aid, &scored_answers).await {
        tracing::error!(error = %e, "Failed to save answers");
        let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
    }

    // 7. Вычисляем затраченное время
    let time_spent = (chrono::Utc::now() - attempt.started_at).num_seconds() as i32;

    // 8. Завершаем попытку в БД
    let completed_attempt = match state
        .attempt_repo
        .complete(tenant_id, aid, scoring_result.total_score, scoring_result.passed, time_spent)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "Failed to complete attempt");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    // 9. (Этап 10.7) Интеграция с прогрессом: если тест сдан, отмечаем урок как завершённый
    if scoring_result.passed {
        let _ = state
            .progress_service
            .update_lesson_progress(
                tenant_id,
                user_id,
                nid,
                Some(LessonStatus::Completed),
                Some(scoring_result.total_score),
                None,
                None,
                None,
            )
            .await;
    }

    let response = ApiResponse::ok(completed_attempt);
    (StatusCode::OK, Json(response)).into_response()
}

/// Получает детали попытки с ответами (для студента или инструктора).
pub async fn get_attempt_details(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Path(attempt_id): Path<Uuid>,
) -> impl IntoResponse {
    let aid = AttemptId(attempt_id);
    tracing::info!(tenant_id = %tenant_id, attempt_id = %aid, "Getting attempt details");

    let attempt = match state.attempt_repo.get_by_id(tenant_id, aid).await {
        Ok(a) => a,
        Err(_) => {
            let response: ApiResponse<serde_json::Value> = ApiResponse::err("Attempt not found");
            return (StatusCode::NOT_FOUND, Json(response)).into_response();
        }
    };

    let answers = match state.attempt_repo.get_answers_for_attempt(tenant_id, aid).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch answers for attempt");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let response = ApiResponse::ok(serde_json::json!({
        "attempt": attempt,
        "answers": answers
    }));
    (StatusCode::OK, Json(response)).into_response()
}

/// Получает историю попыток пользователя по конкретному курсу.
pub async fn list_attempts(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<TenantId>,
    Extension(user_id): Extension<UserId>,
    Path(course_id): Path<Uuid>,
) -> impl IntoResponse {
    let cid = CourseId(course_id);
    tracing::info!(tenant_id = %tenant_id, user_id = %user_id, course_id = %cid, "Listing attempts");

    match state
        .attempt_repo
        .list_by_user_and_course(tenant_id, user_id, cid)
        .await
    {
        Ok(attempts) => {
            let response = ApiResponse::ok(attempts);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list attempts");
            let response: ApiResponse<serde_json::Value> = ApiResponse::err(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_question_request_deserialization() {
        let json = serde_json::json!({
            "title": "What is Rust?",
            "question_type": "ShortAnswer",
            "correct_answer": "A programming language",
            "points": 10,
            "order": 1
        });
        let req: CreateQuestionRequest = serde_json::from_value(json).expect("Must deserialize");
        assert_eq!(req.title, "What is Rust?");
        assert_eq!(req.points, 10);
    }

    #[test]
    fn test_complete_attempt_request_deserialization() {
        let json = serde_json::json!({
            "answers": [
                {
                    "question_id": "00000000-0000-0000-0000-000000000001",
                    "answer_text": "42"
                }
            ]
        });
        let req: CompleteAttemptRequest = serde_json::from_value(json).expect("Must deserialize");
        assert_eq!(req.answers.len(), 1);
        assert_eq!(req.answers[0].answer_text, "42");
    }
}