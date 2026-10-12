// crates/client/src/admin/pages/assessments.rs
//! Управление тестами и вопросами (Этап 10 Assessments Engine).
//!
//! Создание тестов, добавление вопросов разных типов (MultipleChoice, TrueFalse, ShortAnswer),
//! настройка проходного балла, просмотр статистики попыток.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use rust_lms_shared::models::{Attempt, Question, QuestionType};

/// Страница управления тестами курса.
#[component]
pub fn AdminAssessmentList() -> impl IntoView {
    let params = use_params_map();
    // ✅ Leptos 0.8: ParamsMap::get() возвращает Option<String> (владеющий)
    let course_id = move || params.with(|p| p.get("course_id").unwrap_or_default());

    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let questions_resource = Resource::new(course_id, |id| fetch_course_questions(id));
    let attempts_resource = Resource::new(course_id, |id| fetch_course_attempts(id));
    let (show_create_modal, set_show_create_modal) = signal(false);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            <div class="flex items-center justify-between mb-8">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.assessments.title")
                        "Управление тестами"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.assessments.subtitle")
                        "Создавайте вопросы и отслеживайте результаты студентов"
                    </p>
                </div>
                <button
                    on:click=move |_| set_show_create_modal.set(true)
                    class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    // i18n: t!("admin.assessments.add_question")
                    "Добавить вопрос"
                </button>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                // Список вопросов
                <div class="lg:col-span-2">
                    <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">
                        // i18n: t!("admin.assessments.questions_title")
                        "Вопросы теста"
                    </h2>
                    <Suspense fallback=QuestionsSkeleton>
                        {move || {
                            questions_resource.get().map(|result| {
                                match result {
                                    Ok(questions) => {
                                        if questions.is_empty() {
                                            view! {
                                                <div class="text-center py-12 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                                    <p class="text-gray-500 dark:text-gray-400">
                                                        // i18n: t!("admin.assessments.no_questions")
                                                        "Вопросы ещё не добавлены"
                                                    </p>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="space-y-3">
                                                    {questions.into_iter().enumerate().map(|(idx, q)| {
                                                        view! { <QuestionCard index=idx + 1 question=q /> }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            }.into_any()
                                        }
                                    }
                                    Err(e) => view! {
                                        <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                            {e.to_string()}
                                        </div>
                                    }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>

                // Статистика попыток
                <div>
                    <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">
                        // i18n: t!("admin.assessments.stats_title")
                        "Статистика"
                    </h2>
                    <Suspense fallback=StatsSkeleton>
                        {move || {
                            attempts_resource.get().map(|result| {
                                match result {
                                    Ok(attempts) => view! {
                                        <AttemptsStats attempts=attempts />
                                    }.into_any(),
                                    Err(_) => view! { <div></div> }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>
            </div>

            <Show when=move || show_create_modal.get()>
                <CreateQuestionModal
                    course_id=course_id()
                    on_close=move || set_show_create_modal.set(false)
                    on_created=move || {
                        set_show_create_modal.set(false);
                        questions_resource.refetch();
                    }
                />
            </Show>
        </div>
    }
}

/// Карточка вопроса с типом и действиями.
#[component]
fn QuestionCard(index: usize, question: Question) -> impl IntoView {
    let type_badge = match question.question_type {
        QuestionType::MultipleChoice => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "Выбор"),
        QuestionType::TrueFalse => ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "Верно/Неверно"),
        QuestionType::ShortAnswer => ("bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300", "Короткий ответ"),
        QuestionType::LongAnswer => ("bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300", "Длинный ответ"),
    };

    // ✅ Клонируем данные, которые используются в view! более одного раза
    // или внутри замыканий, чтобы избежать E0382 (use of moved value).
    let title = question.title.clone();
    let description = question.description.clone();
    let has_description = description.is_some();
    let description_text = description.unwrap_or_default();
    let points = question.points;

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-4 hover:shadow-md transition-shadow">
            <div class="flex items-start gap-3">
                <div class="flex-shrink-0 w-8 h-8 bg-gray-100 dark:bg-gray-700 rounded-full flex items-center justify-center text-sm font-semibold text-gray-700 dark:text-gray-300">
                    {index}
                </div>
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-2">
                        <span class=format!("inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {}", type_badge.0)>
                            {type_badge.1}
                        </span>
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                            {points} " баллов"
                        </span>
                    </div>
                    <h3 class="font-medium text-gray-900 dark:text-white mb-1">
                        {title}
                    </h3>
                    <Show when=move || has_description>
                        <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-2">
                            {description_text.clone()}
                        </p>
                    </Show>
                </div>
                <div class="flex items-center gap-1">
                    <button class="p-1.5 text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                        </svg>
                    </button>
                    <button class="p-1.5 text-gray-600 dark:text-gray-400 hover:text-red-600 dark:hover:text-red-400 transition-colors">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Блок статистики попыток.
#[component]
fn AttemptsStats(attempts: Vec<Attempt>) -> impl IntoView {
    let total = attempts.len();
    // ✅ passed — Option<bool>; unwrap_or(false)
    let passed = attempts.iter().filter(|a| a.passed.unwrap_or(false)).count();
    let avg_score = if total > 0 {
        attempts.iter().map(|a| a.score.unwrap_or(0.0)).sum::<f64>() / total as f64
    } else {
        0.0
    };

    let passed_percent = if total > 0 {
        format!("{:.1}%", (passed as f64 / total as f64) * 100.0)
    } else {
        "0%".to_string()
    };
    let avg_score_percent = format!("{:.1}%", avg_score * 100.0);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6 space-y-4">
            <div>
                <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">
                    // i18n: t!("admin.assessments.total_attempts")
                    "Всего попыток"
                </div>
                <div class="text-3xl font-bold text-gray-900 dark:text-white">{total}</div>
            </div>
            <div>
                <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">
                    // i18n: t!("admin.assessments.passed_attempts")
                    "Успешно сдано"
                </div>
                <div class="text-3xl font-bold text-green-600 dark:text-green-400">
                    {passed} " / " {total}
                </div>
                <div class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                    {passed_percent}
                </div>
            </div>
            <div>
                <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">
                    // i18n: t!("admin.assessments.avg_score")
                    "Средний балл"
                </div>
                <div class="text-3xl font-bold text-blue-600 dark:text-blue-400">
                    {avg_score_percent}
                </div>
            </div>
        </div>
    }
}

/// Модальное окно создания вопроса.
#[component]
fn CreateQuestionModal(
    course_id: String,
    on_close: impl Fn() + Send + Sync + 'static,
    on_created: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_created = StoredValue::new(on_created);

    let (question_type, set_question_type) = signal(QuestionType::MultipleChoice);
    let (title, set_title) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (points, set_points) = signal(1.0f64);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let course_id = course_id.clone();
        spawn_local(async move {
            let _ = create_question(
                course_id,
                question_type.get(),
                title.get(),
                description.get(),
                points.get(),
            )
            .await;
            on_created.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <h3 class="text-2xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.assessments.create_modal_title")
                        "Создать вопрос"
                    </h3>
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                <form on:submit=on_submit class="p-6 space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                            "Тип вопроса"
                        </label>
                        // ✅ QuestionType не IntoProperty; используем value + on:change
                        <select
                            prop:value=move || question_type.get().to_string()
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_question_type.set(match val.as_str() {
                                    "true_false" => QuestionType::TrueFalse,
                                    "short_answer" => QuestionType::ShortAnswer,
                                    "long_answer" => QuestionType::LongAnswer,
                                    _ => QuestionType::MultipleChoice,
                                });
                            }
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        >
                            <option value="multiple_choice">"Выбор из вариантов"</option>
                            <option value="true_false">"Верно / Неверно"</option>
                            <option value="short_answer">"Короткий ответ"</option>
                            <option value="long_answer">"Длинный ответ"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Вопрос *"
                        </label>
                        <input
                            type="text"
                            prop:value=title
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Описание (опционально)"
                        </label>
                        <textarea
                            prop:value=description
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                            rows="3"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        ></textarea>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Баллы за правильный ответ"
                        </label>
                        <input
                            type="number"
                            step="0.1"
                            min="0"
                            prop:value=points
                            on:input=move |ev| {
                                if let Ok(val) = event_target_value(&ev).parse() {
                                    set_points.set(val);
                                }
                            }
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        />
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700"
                        >
                            "Создать"
                        </button>
                        <button
                            type="button"
                            on:click=move |_| on_close.with_value(|f| f())
                            class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600"
                        >
                            "Отмена"
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

#[component]
fn QuestionsSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-3 animate-pulse">
            <div class="h-24 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
            <div class="h-24 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
            <div class="h-24 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
        </div>
    }
}

#[component]
fn StatsSkeleton() -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 space-y-4 animate-pulse">
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </div>
    }
}

// ============================================================================
// Server-функции (мок-данные для вёрстки)
// ============================================================================

#[server(GetCourseQuestions, "/api")]
pub async fn fetch_course_questions(_course_id: String) -> Result<Vec<Question>, ServerFnError> {
    // TODO: Интеграция с QuestionRepository::list_by_course() из Этапа 10
    // На этапе вёрстки возвращаем мок-вопросы.
    use rust_lms_shared::models::{AnswerOption, CourseId, QuestionId, TenantId};

    let tenant_id = TenantId::new();
    let course_id = CourseId::new();
    let now = chrono::Utc::now();

    let mock = vec![
        Question {
            id: QuestionId::new(),
            tenant_id,
            course_id,
            title: "Какой инструмент управляет версиями Rust?".to_string(),
            description: Some("Выберите один вариант".to_string()),
            question_type: QuestionType::MultipleChoice,
            options: Some(vec![
                AnswerOption { text: "rustup".to_string(), is_correct: true },
                AnswerOption { text: "cargo".to_string(), is_correct: false },
            ]),
            correct_answer: None,
            points: 1,
            order: 1,
            created_at: now,
            updated_at: now,
        },
        Question {
            id: QuestionId::new(),
            tenant_id,
            course_id,
            title: "Rust использует GC для управления памятью.".to_string(),
            description: None,
            question_type: QuestionType::TrueFalse,
            options: None,
            correct_answer: Some("false".to_string()),
            points: 2,
            order: 2,
            created_at: now,
            updated_at: now,
        },
    ];

    Ok(mock)
}

#[server(GetCourseAttempts, "/api")]
pub async fn fetch_course_attempts(_course_id: String) -> Result<Vec<Attempt>, ServerFnError> {
    // TODO: Интеграция с AttemptRepository::list_by_course() из Этапа 10
    // На этапе вёрстки возвращаем пустой список.
    Ok(vec![])
}

#[server(CreateQuestion, "/api")]
pub async fn create_question(
    _course_id: String,
    question_type: QuestionType,
    title: String,
    description: String,
    points: f64,
) -> Result<Question, ServerFnError> {
    // TODO: Интеграция с QuestionRepository::create() из Этапа 10
    // На этапе вёрстки возвращаем мок-вопрос.
    use rust_lms_shared::models::{CourseId, QuestionId, TenantId};

    let now = chrono::Utc::now();
    let question = Question {
        id: QuestionId::new(),
        tenant_id: TenantId::new(),
        course_id: CourseId::new(),
        title,
        description: if description.is_empty() { None } else { Some(description) },
        question_type,
        options: None,
        correct_answer: None,
        points: points as i32,
        order: 1,
        created_at: now,
        updated_at: now,
    };

    Ok(question)
}