// crates/api/src/services/scoring.rs
//! Движок подсчёта баллов для тестов и оценок.
//!
//! Реализует логику проверки ответов на вопросы разных типов:
//! - `MultipleChoice`: сравнение выбранных вариантов с правильными
//! - `TrueFalse`: сравнение булевого значения
//! - `ShortAnswer`: нормализованное сравнение текста
//! - `LongAnswer`: не проверяется автоматически (требует ручной проверки)
//!
//! Движок не хранит состояние и может использоваться как утилита.

use rust_lms_shared::{Question, QuestionType};

/// Результат проверки одного ответа.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoringResult {
    /// Правильный ли ответ.
    pub is_correct: bool,
    /// Заработанные баллы.
    pub points_earned: i32,
    /// Объяснение (для неверных ответов или ручной проверки).
    pub explanation: Option<String>,
}

/// Результат проверки всех ответов попытки.
#[derive(Debug, Clone)]
pub struct AttemptScoringResult {
    /// Результаты по каждому вопросу.
    pub results: Vec<QuestionScoringResult>,
    /// Суммарный балл (0.0–1.0).
    pub total_score: f64,
    /// Суммарные заработанные баллы.
    pub total_points_earned: i32,
    /// Максимально возможные баллы.
    pub total_points_possible: i32,
    /// Сдан ли тест (на основе проходного порога).
    pub passed: bool,
}

/// Результат проверки одного вопроса в попытке.
#[derive(Debug, Clone)]
pub struct QuestionScoringResult {
    /// Идентификатор вопроса.
    pub question_id: rust_lms_shared::QuestionId,
    /// Результат проверки.
    pub scoring: ScoringResult,
}

/// Движок подсчёта баллов.
#[derive(Debug, Clone, Default)]
pub struct ScoringEngine;

impl ScoringEngine {
    /// Создаёт новый экземпляр движка.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Проверяет один ответ на вопрос.
    ///
    /// # Аргументы
    ///
    /// * `question` — вопрос с правильным ответом
    /// * `answer_text` — ответ студента
    ///
    /// # Возвращает
    ///
    /// Результат проверки с баллами и объяснением.
    #[must_use]
    pub fn score_answer(&self, question: &Question, answer_text: &str) -> ScoringResult {
        match question.question_type {
            QuestionType::MultipleChoice => {
                self.score_multiple_choice(question, answer_text)
            }
            QuestionType::TrueFalse => self.score_true_false(question, answer_text),
            QuestionType::ShortAnswer => self.score_short_answer(question, answer_text),
            QuestionType::LongAnswer => self.score_long_answer(question, answer_text),
        }
    }

    /// Проверяет все ответы попытки и вычисляет итоговый балл.
    ///
    /// # Аргументы
    ///
    /// * `questions` — список вопросов
    /// * `answers` — пары (question_id, answer_text)
    /// * `passing_score` — проходной порог (0.0–1.0)
    ///
    /// # Возвращает
    ///
    /// Итоговый результат с суммарным баллом и признаком сдачи.
    #[must_use]
    pub fn score_attempt(
        &self,
        questions: &[Question],
        answers: &[(rust_lms_shared::QuestionId, String)],
        passing_score: f64,
    ) -> AttemptScoringResult {
        let mut results = Vec::with_capacity(questions.len());
        let mut total_points_earned = 0;
        let mut total_points_possible = 0;

        for question in questions {
            // Ищем ответ на этот вопрос
            let answer = answers
                .iter()
                .find(|(qid, _)| *qid == question.id)
                .map(|(_, text)| text.as_str())
                .unwrap_or("");

            let scoring = self.score_answer(question, answer);
            total_points_earned += scoring.points_earned;
            total_points_possible += question.points;

            results.push(QuestionScoringResult {
                question_id: question.id,
                scoring,
            });
        }

        // Вычисляем суммарный балл (0.0–1.0)
        let total_score = if total_points_possible > 0 {
            (total_points_earned as f64) / (total_points_possible as f64)
        } else {
            0.0
        };

        // Определяем сдачу на основе проходного порога
        let passed = total_score >= passing_score;

        AttemptScoringResult {
            results,
            total_score,
            total_points_earned,
            total_points_possible,
            passed,
        }
    }

    /// Проверка ответа типа "Множественный выбор".
    ///
    /// Ответ студента — JSON-массив выбранных индексов или текстов вариантов.
    /// Пример: `"[0, 2]"` или `"[\"Вариант A\", \"Вариант C\"]"`.
    fn score_multiple_choice(&self, question: &Question, answer_text: &str) -> ScoringResult {
        let options = match &question.options {
            Some(opts) => opts,
            None => {
                return ScoringResult {
                    is_correct: false,
                    points_earned: 0,
                    explanation: Some("Вопрос не имеет вариантов ответа".to_string()),
                };
            }
        };

        // Парсим ответ студента как JSON-массив
        let selected: Vec<serde_json::Value> = match serde_json::from_str(answer_text) {
            Ok(v) => v,
            Err(_) => {
                return ScoringResult {
                    is_correct: false,
                    points_earned: 0,
                    explanation: Some("Неверный формат ответа для множественного выбора".to_string()),
                };
            }
        };

        // Извлекаем правильные индексы
        let correct_indices: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, opt)| opt.is_correct)
            .map(|(i, _)| i)
            .collect();

        // Извлекаем выбранные индексы
        let selected_indices: Vec<usize> = selected
            .iter()
            .filter_map(|v| {
                // Поддерживаем как индексы (числа), так и тексты вариантов
                if let Some(idx) = v.as_u64() {
                    Some(idx as usize)
                } else if let Some(text) = v.as_str() {
                    // Ищем вариант по тексту
                    options
                        .iter()
                        .position(|opt| opt.text == text)
                } else {
                    None
                }
            })
            .collect();

        // Проверяем совпадение множеств
        let is_correct = Self::sets_equal(&correct_indices, &selected_indices);

        let points_earned = if is_correct { question.points } else { 0 };

        let explanation = if is_correct {
            None
        } else {
            let correct_texts: Vec<&str> = correct_indices
                .iter()
                .filter_map(|i| options.get(*i).map(|o| o.text.as_str()))
                .collect();
            Some(format!(
                "Правильные ответы: {}",
                correct_texts.join(", ")
            ))
        };

        ScoringResult {
            is_correct,
            points_earned,
            explanation,
        }
    }

    /// Проверка ответа типа "Правда/Ложь".
    ///
    /// Ответ студента — строка `"true"` или `"false"`.
    fn score_true_false(&self, question: &Question, answer_text: &str) -> ScoringResult {
        let correct_answer = match &question.correct_answer {
            Some(ans) => ans.to_lowercase(),
            None => {
                return ScoringResult {
                    is_correct: false,
                    points_earned: 0,
                    explanation: Some("Вопрос не имеет правильного ответа".to_string()),
                };
            }
        };

        let student_answer = answer_text.trim().to_lowercase();

        let is_correct = student_answer == correct_answer;

        let points_earned = if is_correct { question.points } else { 0 };

        let explanation = if is_correct {
            None
        } else {
            Some(format!("Правильный ответ: {}", correct_answer))
        };

        ScoringResult {
            is_correct,
            points_earned,
            explanation,
        }
    }

    /// Проверка ответа типа "Короткий ответ".
    ///
    /// Сравнение с нормализацией: приведение к нижнему регистру,
    /// удаление лишних пробелов, обрезка по краям.
    fn score_short_answer(&self, question: &Question, answer_text: &str) -> ScoringResult {
        let correct_answer = match &question.correct_answer {
            Some(ans) => Self::normalize_text(ans),
            None => {
                return ScoringResult {
                    is_correct: false,
                    points_earned: 0,
                    explanation: Some("Вопрос не имеет правильного ответа".to_string()),
                };
            }
        };

        let student_answer = Self::normalize_text(answer_text);

        let is_correct = student_answer == correct_answer;

        let points_earned = if is_correct { question.points } else { 0 };

        let explanation = if is_correct {
            None
        } else {
            Some(format!("Правильный ответ: {}", correct_answer))
        };

        ScoringResult {
            is_correct,
            points_earned,
            explanation,
        }
    }

    /// Проверка ответа типа "Развёрнутый ответ".
    ///
    /// Не проверяется автоматически — требует ручной проверки инструктором.
    /// Возвращает `is_correct = false` и `points_earned = 0` до ручной проверки.
    fn score_long_answer(&self, _question: &Question, _answer_text: &str) -> ScoringResult {
        ScoringResult {
            is_correct: false,
            points_earned: 0,
            explanation: Some(
                "Развёрнутый ответ требует ручной проверки инструктором".to_string(),
            ),
        }
    }

    /// Нормализует текст для сравнения:
    /// - Приведение к нижнему регистру
    /// - Удаление лишних пробелов
    /// - Обрезка по краям
    #[must_use]
    fn normalize_text(text: &str) -> String {
        text.to_lowercase()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// Проверяет равенство двух множеств индексов.
    #[must_use]
    fn sets_equal(a: &[usize], b: &[usize]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut a_sorted = a.to_vec();
        let mut b_sorted = b.to_vec();
        a_sorted.sort();
        b_sorted.sort();

        a_sorted == b_sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use rust_lms_shared::{AnswerOption, CourseId, QuestionId, QuestionType, TenantId};
    use uuid::Uuid;

    /// Вспомогательная функция для создания тестового вопроса.
    fn create_test_question(
        question_type: QuestionType,
        options: Option<Vec<AnswerOption>>,
        correct_answer: Option<String>,
        points: i32,
    ) -> Question {
        let now: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        Question {
            id: QuestionId(Uuid::new_v4()),
            tenant_id: TenantId(Uuid::new_v4()),
            course_id: CourseId(Uuid::new_v4()),
            title: "Test Question".to_string(),
            description: None,
            question_type,
            options,
            correct_answer,
            points,
            order: 1,
            created_at: now,
            updated_at: now,
        }
    }

    // ========================================================================
    // MultipleChoice
    // ========================================================================

    #[test]
    fn test_multiple_choice_correct_by_indices() {
        let options = vec![
            AnswerOption {
                text: "Вариант A".to_string(),
                is_correct: true,
            },
            AnswerOption {
                text: "Вариант B".to_string(),
                is_correct: false,
            },
            AnswerOption {
                text: "Вариант C".to_string(),
                is_correct: true,
            },
        ];

        let question = create_test_question(
            QuestionType::MultipleChoice,
            Some(options),
            None,
            5,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "[0, 2]");

        assert!(result.is_correct);
        assert_eq!(result.points_earned, 5);
        assert!(result.explanation.is_none());
    }

    #[test]
    fn test_multiple_choice_correct_by_texts() {
        let options = vec![
            AnswerOption {
                text: "Вариант A".to_string(),
                is_correct: true,
            },
            AnswerOption {
                text: "Вариант B".to_string(),
                is_correct: false,
            },
            AnswerOption {
                text: "Вариант C".to_string(),
                is_correct: true,
            },
        ];

        let question = create_test_question(
            QuestionType::MultipleChoice,
            Some(options),
            None,
            5,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, r#"["Вариант A", "Вариант C"]"#);

        assert!(result.is_correct);
        assert_eq!(result.points_earned, 5);
    }

    #[test]
    fn test_multiple_choice_incorrect() {
        let options = vec![
            AnswerOption {
                text: "Вариант A".to_string(),
                is_correct: true,
            },
            AnswerOption {
                text: "Вариант B".to_string(),
                is_correct: false,
            },
        ];

        let question = create_test_question(
            QuestionType::MultipleChoice,
            Some(options),
            None,
            3,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "[1]");

        assert!(!result.is_correct);
        assert_eq!(result.points_earned, 0);
        assert!(result.explanation.is_some());
        assert!(result.explanation.unwrap().contains("Вариант A"));
    }

    #[test]
    fn test_multiple_choice_no_options() {
        let question = create_test_question(
            QuestionType::MultipleChoice,
            None, // Нет вариантов ответа
            None,
            3,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "[0]");

        assert!(!result.is_correct);
        assert_eq!(result.points_earned, 0);
        assert!(result.explanation.is_some());
        assert!(result.explanation.unwrap().contains("не имеет вариантов"));
    }

    #[test]
    fn test_multiple_choice_invalid_json() {
        let options = vec![AnswerOption {
            text: "Вариант A".to_string(),
            is_correct: true,
        }];

        let question = create_test_question(
            QuestionType::MultipleChoice,
            Some(options),
            None,
            3,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "not a json");

        assert!(!result.is_correct);
        assert_eq!(result.points_earned, 0);
        assert!(result.explanation.is_some());
        assert!(result.explanation.unwrap().contains("Неверный формат"));
    }

    // ========================================================================
    // TrueFalse
    // ========================================================================

    #[test]
    fn test_true_false_correct() {
        let question = create_test_question(
            QuestionType::TrueFalse,
            None,
            Some("true".to_string()),
            2,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "true");

        assert!(result.is_correct);
        assert_eq!(result.points_earned, 2);
    }

    #[test]
    fn test_true_false_case_insensitive() {
        let question = create_test_question(
            QuestionType::TrueFalse,
            None,
            Some("true".to_string()),
            2,
        );

        let engine = ScoringEngine::new();

        let result = engine.score_answer(&question, "TRUE");
        assert!(result.is_correct);

        let result = engine.score_answer(&question, "True");
        assert!(result.is_correct);

        let result = engine.score_answer(&question, "  true  ");
        assert!(result.is_correct);
    }

    #[test]
    fn test_true_false_incorrect() {
        let question = create_test_question(
            QuestionType::TrueFalse,
            None,
            Some("true".to_string()),
            2,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "false");

        assert!(!result.is_correct);
        assert_eq!(result.points_earned, 0);
        assert!(result.explanation.is_some());
    }

    #[test]
    fn test_true_false_no_correct_answer() {
        let question = create_test_question(
            QuestionType::TrueFalse,
            None,
            None, // Нет правильного ответа
            2,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "true");

        assert!(!result.is_correct);
        assert_eq!(result.points_earned, 0);
    }

    // ========================================================================
    // ShortAnswer
    // ========================================================================

    #[test]
    fn test_short_answer_exact_match() {
        let question = create_test_question(
            QuestionType::ShortAnswer,
            None,
            Some("Paris".to_string()),
            3,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "Paris");

        assert!(result.is_correct);
        assert_eq!(result.points_earned, 3);
    }

    #[test]
    fn test_short_answer_normalization() {
        let question = create_test_question(
            QuestionType::ShortAnswer,
            None,
            Some("Paris".to_string()),
            3,
        );

        let engine = ScoringEngine::new();

        // Регистр
        let result = engine.score_answer(&question, "PARIS");
        assert!(result.is_correct);

        // Пробелы по краям
        let result = engine.score_answer(&question, "  Paris  ");
        assert!(result.is_correct);
    }

    #[test]
    fn test_short_answer_multi_word_normalization() {
        let question = create_test_question(
            QuestionType::ShortAnswer,
            None,
            Some("New York".to_string()),
            3,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "  new   york  ");

        assert!(result.is_correct);
    }

    #[test]
    fn test_short_answer_incorrect() {
        let question = create_test_question(
            QuestionType::ShortAnswer,
            None,
            Some("Paris".to_string()),
            3,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "London");

        assert!(!result.is_correct);
        assert_eq!(result.points_earned, 0);
        assert!(result.explanation.is_some());
        assert!(result.explanation.unwrap().contains("paris"));
    }

    // ========================================================================
    // LongAnswer
    // ========================================================================

    #[test]
    fn test_long_answer_requires_manual_review() {
        let question = create_test_question(
            QuestionType::LongAnswer,
            None,
            None,
            10,
        );

        let engine = ScoringEngine::new();
        let result = engine.score_answer(&question, "Это развёрнутый ответ студента.");

        assert!(!result.is_correct);
        assert_eq!(result.points_earned, 0);
        assert!(result.explanation.is_some());
        assert!(result.explanation.unwrap().contains("ручной проверки"));
    }

    // ========================================================================
    // score_attempt
    // ========================================================================

    #[test]
    fn test_score_attempt_all_correct() {
        let q1 = create_test_question(
            QuestionType::TrueFalse,
            None,
            Some("true".to_string()),
            2,
        );
        let q2 = create_test_question(
            QuestionType::ShortAnswer,
            None,
            Some("42".to_string()),
            3,
        );

        let questions = vec![q1.clone(), q2.clone()];
        let answers = vec![
            (q1.id, "true".to_string()),
            (q2.id, "42".to_string()),
        ];

        let engine = ScoringEngine::new();
        let result = engine.score_attempt(&questions, &answers, 0.8);

        assert_eq!(result.total_points_earned, 5);
        assert_eq!(result.total_points_possible, 5);
        assert!((result.total_score - 1.0).abs() < f64::EPSILON);
        assert!(result.passed);
        assert_eq!(result.results.len(), 2);
    }

    #[test]
    fn test_score_attempt_partial_correct() {
        let q1 = create_test_question(
            QuestionType::TrueFalse,
            None,
            Some("true".to_string()),
            2,
        );
        let q2 = create_test_question(
            QuestionType::ShortAnswer,
            None,
            Some("42".to_string()),
            3,
        );

        let questions = vec![q1.clone(), q2.clone()];
        let answers = vec![
            (q1.id, "false".to_string()), // Неправильный ответ
            (q2.id, "42".to_string()),    // Правильный ответ
        ];

        let engine = ScoringEngine::new();
        let result = engine.score_attempt(&questions, &answers, 0.8);

        assert_eq!(result.total_points_earned, 3);
        assert_eq!(result.total_points_possible, 5);
        assert!((result.total_score - 0.6).abs() < f64::EPSILON);
        assert!(!result.passed); // 0.6 < 0.8
    }

    #[test]
    fn test_score_attempt_missing_answer() {
        let q1 = create_test_question(
            QuestionType::TrueFalse,
            None,
            Some("true".to_string()),
            2,
        );
        let q2 = create_test_question(
            QuestionType::ShortAnswer,
            None,
            Some("42".to_string()),
            3,
        );

        let questions = vec![q1.clone(), q2.clone()];
        // Ответ только на первый вопрос
        let answers = vec![(q1.id, "true".to_string())];

        let engine = ScoringEngine::new();
        let result = engine.score_attempt(&questions, &answers, 0.5);

        // Второй вопрос считается неотвеченным (пустой ответ)
        assert_eq!(result.total_points_earned, 2);
        assert_eq!(result.total_points_possible, 5);
        assert!((result.total_score - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn test_score_attempt_empty_questions() {
        let questions: Vec<Question> = vec![];
        let answers: Vec<(QuestionId, String)> = vec![];

        let engine = ScoringEngine::new();
        let result = engine.score_attempt(&questions, &answers, 0.5);

        assert_eq!(result.total_points_earned, 0);
        assert_eq!(result.total_points_possible, 0);
        assert!((result.total_score - 0.0).abs() < f64::EPSILON);
        assert!(!result.passed);
    }

    // ========================================================================
    // normalize_text
    // ========================================================================

    #[test]
    fn test_normalize_text_basic() {
        assert_eq!(ScoringEngine::normalize_text("  Hello   World  "), "hello world");
        assert_eq!(ScoringEngine::normalize_text("UPPERCASE"), "uppercase");
        assert_eq!(ScoringEngine::normalize_text("  spaces   between  "), "spaces between");
    }

    #[test]
    fn test_normalize_text_empty() {
        assert_eq!(ScoringEngine::normalize_text(""), "");
        assert_eq!(ScoringEngine::normalize_text("   "), "");
        assert_eq!(ScoringEngine::normalize_text("\t\n"), "");
    }

    // ========================================================================
    // sets_equal
    // ========================================================================

    #[test]
    fn test_sets_equal_same_order() {
        assert!(ScoringEngine::sets_equal(&[1, 2, 3], &[1, 2, 3]));
    }

    #[test]
    fn test_sets_equal_different_order() {
        assert!(ScoringEngine::sets_equal(&[3, 1, 2], &[2, 3, 1]));
    }

    #[test]
    fn test_sets_equal_different_length() {
        assert!(!ScoringEngine::sets_equal(&[1, 2], &[1, 2, 3]));
        assert!(!ScoringEngine::sets_equal(&[1, 2, 3], &[1, 2]));
    }

    #[test]
    fn test_sets_equal_empty() {
        assert!(ScoringEngine::sets_equal(&[], &[]));
    }

    #[test]
    fn test_sets_equal_different_elements() {
        assert!(!ScoringEngine::sets_equal(&[1, 2, 3], &[1, 2, 4]));
    }
}