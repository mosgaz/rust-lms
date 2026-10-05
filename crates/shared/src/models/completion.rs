//! Критерии завершения курса (CompletionCriteria).

use serde::{Deserialize, Serialize};
use std::fmt;

use super::node::NodeId;

/// Режим проверки критериев завершения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionMode {
    /// Все правила должны быть выполнены.
    AllOf,
    /// Хотя бы одно правило должно быть выполнено.
    AnyOf,
}

impl Default for CompletionMode {
    fn default() -> Self {
        Self::AllOf
    }
}

impl fmt::Display for CompletionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllOf => f.write_str("all_of"),
            Self::AnyOf => f.write_str("any_of"),
        }
    }
}

/// Правило завершения курса.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompletionRule {
    /// Минимальный общий прогресс курса (0.0–1.0).
    MinProgress {
        /// Пороговое значение прогресса.
        value: f64,
    },
    /// Минимальный средний балл по всем тестам курса (0.0–1.0).
    MinAvgQuizScore {
        /// Пороговое значение среднего балла.
        value: f64,
    },
    /// Список обязательных уроков, которые должны быть завершены.
    RequiredNodes {
        /// Идентификаторы обязательных уроков.
        node_ids: Vec<NodeId>,
    },
    /// Все уроки курса должны быть завершены.
    AllLessonsCompleted,
}

impl CompletionRule {
    /// Валидирует правило на корректность значений.
    pub fn validate(&self) -> Result<(), CompletionCriteriaError> {
        match self {
            Self::MinProgress { value } | Self::MinAvgQuizScore { value } => {
                if !(0.0..=1.0).contains(value) {
                    return Err(CompletionCriteriaError::InvalidValue {
                        rule_type: match self {
                            Self::MinProgress { .. } => "min_progress",
                            Self::MinAvgQuizScore { .. } => "min_avg_quiz_score",
                            _ => unreachable!(),
                        },
                        value: *value,
                        reason: "value must be between 0.0 and 1.0".to_string(),
                    });
                }
                Ok(())
            }
            Self::RequiredNodes { node_ids } => {
                if node_ids.is_empty() {
                    return Err(CompletionCriteriaError::InvalidValue {
                        rule_type: "required_nodes",
                        value: 0.0,
                        reason: "node_ids must not be empty".to_string(),
                    });
                }
                Ok(())
            }
            Self::AllLessonsCompleted => Ok(()),
        }
    }
}

impl fmt::Display for CompletionRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MinProgress { value } => write!(f, "min_progress({value})"),
            Self::MinAvgQuizScore { value } => write!(f, "min_avg_quiz_score({value})"),
            Self::RequiredNodes { node_ids } => write!(f, "required_nodes({} nodes)", node_ids.len()),
            Self::AllLessonsCompleted => f.write_str("all_lessons_completed"),
        }
    }
}

/// Критерии завершения курса.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionCriteria {
    /// Режим проверки правил (all_of / any_of).
    #[serde(default)]
    pub mode: CompletionMode,
    /// Список правил, которые должны быть выполнены.
    pub rules: Vec<CompletionRule>,
}

impl CompletionCriteria {
    /// Валидирует все правила критериев.
    pub fn validate(&self) -> Result<(), CompletionCriteriaError> {
        if self.rules.is_empty() {
            return Err(CompletionCriteriaError::EmptyRules);
        }
        for rule in &self.rules {
            rule.validate()?;
        }
        Ok(())
    }

    /// Парсит JSONB-значение в строго типизированную структуру.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, CompletionCriteriaError> {
        let criteria: Self = serde_json::from_value(value.clone()).map_err(|e| {
            CompletionCriteriaError::ParseError(format!("failed to parse completion criteria: {e}"))
        })?;
        criteria.validate()?;
        Ok(criteria)
    }

    /// Сериализует критерии в JSONB-значение.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

impl Default for CompletionCriteria {
    fn default() -> Self {
        Self {
            mode: CompletionMode::AllOf,
            rules: vec![CompletionRule::AllLessonsCompleted],
        }
    }
}

impl fmt::Display for CompletionCriteria {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.mode, self.rules.len())
    }
}

/// Ошибки валидации критериев завершения.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionCriteriaError {
    /// Список правил пуст.
    EmptyRules,
    /// Значение в правиле выходит за допустимые пределы.
    InvalidValue {
        /// Тип правила.
        rule_type: &'static str,
        /// Невалидное значение.
        value: f64,
        /// Причина ошибки.
        reason: String,
    },
    /// Ошибка парсинга JSON.
    ParseError(String),
}

impl fmt::Display for CompletionCriteriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRules => f.write_str("completion criteria must have at least one rule"),
            Self::InvalidValue { rule_type, value, reason } => {
                write!(f, "invalid value {value} for rule {rule_type}: {reason}")
            }
            Self::ParseError(msg) => write!(f, "completion criteria parse error: {msg}"),
        }
    }
}

impl std::error::Error for CompletionCriteriaError {}