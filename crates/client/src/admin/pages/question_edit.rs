// crates/client/src/admin/pages/question_edit.rs
//! Страница редактирования вопроса теста (Этап 10 Assessments Engine).
//!
//! Полное редактирование вопроса: тип (MultipleChoice/TrueFalse/ShortAnswer/LongAnswer),
//! динамический список вариантов ответов, баллы, порядок.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ В `shared::Question` НЕТ поля `explanation`. UI-блок «Объяснение» убран.
//! ⚠️ В `shared::AnswerOption` только `text` и `is_correct` — без `id`, `weight`, `explanation`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_lms_shared::models::{AnswerOption, Question, QuestionType};

/// Страница редактирования вопроса.
#[component]
pub fn AdminQuestionEdit() -> impl IntoView {
    let params = use_params_map();
    // ✅ Leptos 0.8: ParamsMap::get() возвращает Option<String>
    let question_id = move || params.with(|p| p.get("question_id").unwrap_or_default());

    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let question_resource = Resource::new(question_id, |id| fetch_question_for_edit(id));
    let (save_status, set_save_status) = signal(SaveStatus::Idle);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-4xl">
            <Suspense fallback=PageSkeleton>
                {move || {
                    question_resource.get().map(|result| {
                        match result {
                            Ok(question) => view! {
                                <QuestionEditForm
                                    question=question
                                    save_status=save_status.into()
                                    on_save_status_change=move |s| set_save_status.set(s)
                                />
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold mb-1">"Ошибка загрузки вопроса"</p>
                                    <p class="text-sm">{e.to_string()}</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

/// Статус сохранения.
///
/// ⚠️ Не `Copy` из-за `Error(String)`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SaveStatus {
    /// Ожидание изменений.
    Idle,
    /// Идёт сохранение.
    Saving,
    /// Успешно сохранено.
    Saved,
    /// Ошибка сохранения.
    Error(String),
}

/// Основная форма редактирования вопроса.
#[component]
fn QuestionEditForm(
    question: Question,
    save_status: Signal<SaveStatus>,
    on_save_status_change: impl Fn(SaveStatus) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_save_status_change = StoredValue::new(on_save_status_change);

    // ✅ Клонируем поля Question до использования в замыканиях
    let question_id = question.id;
    let course_id_str = question.course_id.0.to_string();
    let question_id_short: String = question.id.to_string().chars().take(8).collect();

    // Базовые сигналы
    let (question_type, set_question_type) = signal(question.question_type);
    let (title, set_title) = signal(question.title.clone());
    let (description, set_description) = signal(question.description.clone().unwrap_or_default());
    let (points, set_points) = signal(question.points);
    let (order, set_order) = signal(question.order);

    // Динамический список вариантов ответов (для MultipleChoice)
    let initial_options = question.options.clone().unwrap_or_default();
    let (options, set_options) = signal(initial_options);

    // Для TrueFalse
    let (true_false_answer, set_true_false_answer) =
        signal(question.correct_answer.as_deref() == Some("true"));

    // Для ShortAnswer/LongAnswer
    let (text_answer, set_text_answer) =
        signal(question.correct_answer.clone().unwrap_or_default());

    // Обработчик сохранения
    let handle_save = move || {
        on_save_status_change.with_value(|f| f(SaveStatus::Saving));

        let correct_answer = match question_type.get() {
            QuestionType::TrueFalse => Some(if true_false_answer.get() {
                "true".to_string()
            } else {
                "false".to_string()
            }),
            QuestionType::ShortAnswer | QuestionType::LongAnswer => {
                let val = text_answer.get();
                if val.is_empty() { None } else { Some(val) }
            }
            QuestionType::MultipleChoice => None,
        };

        let payload = QuestionUpdatePayload {
            id: question_id.0,
            question_type: question_type.get(),
            title: title.get(),
            description: description.get(),
            points: points.get(),
            order: order.get(),
            correct_answer,
            options: if question_type.get() == QuestionType::MultipleChoice {
                Some(options.get())
            } else {
                None
            },
        };

        let on_save_status_change = on_save_status_change;
        spawn_local(async move {
            match update_question(payload).await {
                Ok(_) => on_save_status_change.with_value(|f| f(SaveStatus::Saved)),
                Err(e) => on_save_status_change
                    .with_value(|f| f(SaveStatus::Error(e.to_string()))),
            }
        });
    };

    // Обработчик добавления варианта ответа
    let add_option = move || {
        let mut current = options.get();
        current.push(AnswerOption {
            text: String::new(),
            is_correct: false,
        });
        set_options.set(current);
    };

    view! {
        <div>
            // Верхняя панель
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div class="flex items-center gap-3">
                    <a
                        href=format!("/admin/courses/{}/assessments", course_id_str)
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path>
                        </svg>
                    </a>
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">
                            "Редактирование вопроса"
                        </h1>
                        <p class="text-sm text-gray-500 dark:text-gray-400">
                            "ID: " {question_id_short} "..."
                        </p>
                    </div>
                </div>

                <div class="flex items-center gap-2">
                    <Show when=move || save_status.get() == SaveStatus::Saved>
                        <span class="text-sm text-green-600 dark:text-green-400 flex items-center gap-1">
                            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
                            </svg>
                            "Сохранено"
                        </span>
                    </Show>
                    <button
                        on:click=move |_| handle_save()
                        disabled=move || save_status.get() == SaveStatus::Saving
                        class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    >
                        {move || {
                            if save_status.get() == SaveStatus::Saving { "Сохранение..." } else { "Сохранить" }
                        }}
                    </button>
                </div>
            </div>

            <div class="space-y-6">
                // 1. Тип вопроса
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                        "Тип вопроса"
                    </label>
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                        <QuestionTypeCard
                            icon="🔘"
                            label="Выбор из вариантов"
                            description="Multiple choice"
                            active=move || question_type.get() == QuestionType::MultipleChoice
                            on_click=move || set_question_type.set(QuestionType::MultipleChoice)
                        />
                        <QuestionTypeCard
                            icon="✓✗"
                            label="Верно / Неверно"
                            description="True or False"
                            active=move || question_type.get() == QuestionType::TrueFalse
                            on_click=move || set_question_type.set(QuestionType::TrueFalse)
                        />
                        <QuestionTypeCard
                            icon="📝"
                            label="Короткий ответ"
                            description="Short answer"
                            active=move || question_type.get() == QuestionType::ShortAnswer
                            on_click=move || set_question_type.set(QuestionType::ShortAnswer)
                        />
                        <QuestionTypeCard
                            icon="📄"
                            label="Длинный ответ"
                            description="Essay (ручная проверка)"
                            active=move || question_type.get() == QuestionType::LongAnswer
                            on_click=move || set_question_type.set(QuestionType::LongAnswer)
                        />
                    </div>
                </div>

                // 2. Текст вопроса
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        "Текст вопроса *"
                    </label>
                    <input
                        type="text"
                        prop:value=title
                        on:input=move |ev| set_title.set(event_target_value(&ev))
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 text-lg"
                        placeholder="Введите текст вопроса..."
                        required
                    />
                    <div class="mt-3">
                        <label class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1">
                            "Подсказка / контекст (опционально)"
                        </label>
                        <textarea
                            prop:value=description
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                            rows="2"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 text-sm"
                            placeholder="Дополнительная информация для студента..."
                        ></textarea>
                    </div>
                </div>

                // 3. Варианты ответа / правильный ответ
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                        "Ответы"
                    </h3>

                    {move || {
                        match question_type.get() {
                            QuestionType::MultipleChoice => view! {
                                <MultipleChoiceEditor
                                    options=options.into()
                                    set_options=set_options
                                    on_add_option=add_option
                                />
                            }.into_any(),
                            QuestionType::TrueFalse => view! {
                                <TrueFalseEditor
                                    answer=true_false_answer.into()
                                    set_answer=set_true_false_answer
                                />
                            }.into_any(),
                            QuestionType::ShortAnswer => view! {
                                <ShortAnswerEditor
                                    answer=text_answer.into()
                                    set_answer=set_text_answer
                                    multiline=false
                                />
                            }.into_any(),
                            QuestionType::LongAnswer => view! {
                                <ShortAnswerEditor
                                    answer=text_answer.into()
                                    set_answer=set_text_answer
                                    multiline=true
                                />
                            }.into_any(),
                        }
                    }}
                </div>

                // 4. Настройки (баллы и порядок)
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                        "Настройки"
                    </h3>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                "Баллы за правильный ответ"
                            </label>
                            <input
                                type="number"
                                step="1"
                                min="0"
                                prop:value=points
                                on:input=move |ev| {
                                    if let Ok(val) = event_target_value(&ev).parse() {
                                        set_points.set(val);
                                    }
                                }
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                "Порядок в тесте"
                            </label>
                            <input
                                type="number"
                                min="1"
                                prop:value=order
                                on:input=move |ev| {
                                    if let Ok(val) = event_target_value(&ev).parse() {
                                        set_order.set(val);
                                    }
                                }
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                            />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Карточка выбора типа вопроса.
#[component]
fn QuestionTypeCard(
    icon: &'static str,
    label: &'static str,
    description: &'static str,
    active: impl Fn() -> bool + Send + Sync + 'static,
    on_click: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let active = StoredValue::new(active);
    let on_click = StoredValue::new(on_click);

    view! {
        <button
            on:click=move |_| on_click.with_value(|f| f())
            class=move || {
                format!(
                    "p-4 border-2 rounded-lg text-left transition-all {}",
                    if active.with_value(|f| f()) {
                        "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                    } else {
                        "border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600"
                    }
                )
            }
        >
            <div class="text-2xl mb-2">{icon}</div>
            <div class="font-semibold text-gray-900 dark:text-white text-sm">{label}</div>
            <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{description}</div>
        </button>
    }
}

/// Редактор вариантов ответа для MultipleChoice.
#[component]
fn MultipleChoiceEditor(
    options: Signal<Vec<AnswerOption>>,
    set_options: WriteSignal<Vec<AnswerOption>>,
    on_add_option: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_add_option = StoredValue::new(on_add_option);

    let correct_count = move || options.get().iter().filter(|o| o.is_correct).count();
    let has_correct = move || correct_count() > 0;

    view! {
        <div class="space-y-3">
            <div class="flex items-center justify-between">
                <p class="text-sm text-gray-600 dark:text-gray-400">
                    "Отметьте правильные варианты. Можно выбрать несколько."
                </p>
                <span class=move || {
                    format!(
                        "text-xs px-2 py-1 rounded-full {}",
                        if has_correct() {
                            "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300"
                        } else {
                            "bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300"
                        }
                    )
                }>
                    // ✅ String вместо &format!() — иначе E0515
                    {move || {
                        let count = correct_count();
                        if count == 0 {
                            "Нет правильных".to_string()
                        } else {
                            format!("{} правильных", count)
                        }
                    }}
                </span>
            </div>

            {move || {
                options.get().into_iter().enumerate().map(|(idx, option)| {
                    let option_clone = option.clone();
                    view! {
                        <OptionRow
                            index=idx
                            option=option_clone
                            on_change=move |updated| {
                                let mut current = options.get();
                                current[idx] = updated;
                                set_options.set(current);
                            }
                            on_remove=move || {
                                let mut current = options.get();
                                current.remove(idx);
                                set_options.set(current);
                            }
                        />
                    }
                }).collect::<Vec<_>>()
            }}

            <button
                on:click=move |_| on_add_option.with_value(|f| f())
                class="w-full py-3 border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg text-gray-600 dark:text-gray-400 hover:border-blue-400 dark:hover:border-blue-500 hover:text-blue-600 dark:hover:text-blue-400 transition-colors flex items-center justify-center gap-2"
            >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                </svg>
                "Добавить вариант"
            </button>
        </div>
    }
}

/// Строка одного варианта ответа.
#[component]
fn OptionRow(
    index: usize,
    option: AnswerOption,
    on_change: impl Fn(AnswerOption) + Send + Sync + 'static,
    on_remove: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_change = StoredValue::new(on_change);
    let on_remove = StoredValue::new(on_remove);

    let letter = (b'A' + index as u8) as char;
    let is_correct = option.is_correct;
    let option_text = option.text.clone();

    // ✅ Клонируем option для каждого move-замыкания (AnswerOption не Copy)
    let option_for_check = option.clone();
    let option_for_input = option.clone();

    let bg_class = if is_correct {
        "border-green-300 dark:border-green-700 bg-green-50/50 dark:bg-green-900/10"
    } else {
        "border-gray-200 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-800/50"
    };

    view! {
        <div class=format!("flex items-start gap-3 p-3 border rounded-lg transition-all {}", bg_class)>
            // Чекбокс "Правильный ответ"
            <label class="relative inline-flex items-center cursor-pointer flex-shrink-0 mt-2" title="Отметить как правильный">
                <input
                    type="checkbox"
                    prop:checked=is_correct
                    on:change=move |ev| {
                        let checked = event_target_checked(&ev);
                        let mut updated = option_for_check.clone();
                        updated.is_correct = checked;
                        on_change.with_value(|f| f(updated));
                    }
                    class="sr-only peer"
                />
                <div class="w-6 h-6 bg-gray-200 ... peer-checked:bg-green-600"></div>
            </label>

            // Поле текста варианта
            <div class="flex-1">
                <div class="flex items-center gap-2">
                    <span class="text-xs font-bold text-gray-500 dark:text-gray-400 w-5">
                        {letter.to_string()} "."
                    </span>
                    <input
                        type="text"
                        prop:value=option_text
                        on:input=move |ev| {
                            let text = event_target_value(&ev);
                            let mut updated = option_for_input.clone();
                            updated.text = text;
                            on_change.with_value(|f| f(updated));
                        }
                        class="flex-1 px-2 py-1 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm focus:ring-2 focus:ring-blue-500"
                        placeholder="Текст варианта ответа..."
                    />
                </div>
            </div>

            // Кнопка удаления
            <button
                on:click=move |_| on_remove.with_value(|f| f())
                class="p-1.5 text-gray-400 hover:text-red-600 dark:hover:text-red-400 transition-colors flex-shrink-0"
                title="Удалить вариант"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                </svg>
            </button>
        </div>
    }
}

/// Редактор для True/False вопроса.
#[component]
fn TrueFalseEditor(
    answer: Signal<bool>,
    set_answer: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-2 gap-4">
            <label class="cursor-pointer">
                <input
                    type="radio"
                    name="true_false"
                    prop:checked=move || answer.get()
                    on:change=move |_| set_answer.set(true)
                    class="sr-only peer"
                />
                <div class=move || {
                    format!(
                        "p-6 border-2 rounded-lg text-center transition-all {}",
                        if answer.get() {
                            "border-green-500 bg-green-50 dark:bg-green-900/20"
                        } else {
                            "border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600"
                        }
                    )
                }>
                    <div class="text-4xl mb-2">"✓"</div>
                    <div class="font-semibold text-gray-900 dark:text-white">"Верно"</div>
                </div>
            </label>
            <label class="cursor-pointer">
                <input
                    type="radio"
                    name="true_false"
                    prop:checked=move || !answer.get()
                    on:change=move |_| set_answer.set(false)
                    class="sr-only peer"
                />
                <div class=move || {
                    format!(
                        "p-6 border-2 rounded-lg text-center transition-all {}",
                        if !answer.get() {
                            "border-red-500 bg-red-50 dark:bg-red-900/20"
                        } else {
                            "border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600"
                        }
                    )
                }>
                    <div class="text-4xl mb-2">"✗"</div>
                    <div class="font-semibold text-gray-900 dark:text-white">"Неверно"</div>
                </div>
            </label>
        </div>
    }
}

/// Редактор для ShortAnswer/LongAnswer.
#[component]
fn ShortAnswerEditor(
    answer: Signal<String>,
    set_answer: WriteSignal<String>,
    multiline: bool,
) -> impl IntoView {
    view! {
        <div>
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">
                "Правильный ответ (нормализация: регистр и лишние пробелы игнорируются)"
            </p>
            <Show when=move || !multiline>
                <input
                    type="text"
                    prop:value=answer
                    on:input=move |ev| set_answer.set(event_target_value(&ev))
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                    placeholder="Введите правильный ответ..."
                />
            </Show>
            <Show when=move || multiline>
                <textarea
                    prop:value=answer
                    on:input=move |ev| set_answer.set(event_target_value(&ev))
                    rows="4"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                    placeholder="Примерный правильный ответ (для ручной проверки)..."
                ></textarea>
            </Show>
            <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">
                {if multiline {
                    "Длинные ответы проверяются вручную инструктором"
                } else {
                    "Для коротких ответов можно указать несколько вариантов через запятую"
                }}
            </p>
        </div>
    }
}

#[component]
fn PageSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-6 animate-pulse">
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-24 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Payload для обновления вопроса.
///
/// ⚠️ `points: i32` — в `shared::Question` это `i32`, а не `f64`.
/// ⚠️ Поля `explanation` нет в `shared::Question` — убрано.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionUpdatePayload {
    /// ID вопроса.
    pub id: Uuid,
    /// Тип вопроса.
    pub question_type: QuestionType,
    /// Текст вопроса.
    pub title: String,
    /// Подсказка/контекст.
    pub description: String,
    /// Баллы (i32, как в shared::Question).
    pub points: i32,
    /// Порядок в тесте.
    pub order: i32,
    /// Правильный ответ для TrueFalse/ShortAnswer/LongAnswer.
    pub correct_answer: Option<String>,
    /// Варианты для MultipleChoice.
    pub options: Option<Vec<AnswerOption>>,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

/// Получение вопроса для редактирования.
#[server(GetQuestionForEdit, "/api")]
pub async fn fetch_question_for_edit(_question_id: String) -> Result<Question, ServerFnError> {
    // TODO: Интеграция с QuestionRepository::find_by_id() из Этапа 10
    // На этапе вёрстки возвращаем мок-вопрос.
    use rust_lms_shared::models::{CourseId, QuestionId, TenantId};

    let now = chrono::Utc::now();

    let question = Question {
        id: QuestionId::new(),
        tenant_id: TenantId::new(),
        course_id: CourseId::new(),
        title: "Какой инструмент управляет версиями Rust?".to_string(),
        description: Some("Выберите один правильный вариант".to_string()),
        question_type: QuestionType::MultipleChoice,
        options: Some(vec![
            AnswerOption {
                text: "rustup".to_string(),
                is_correct: true,
            },
            AnswerOption {
                text: "cargo".to_string(),
                is_correct: false,
            },
            AnswerOption {
                text: "clippy".to_string(),
                is_correct: false,
            },
            AnswerOption {
                text: "rustc".to_string(),
                is_correct: false,
            },
        ]),
        correct_answer: None,
        points: 1,
        order: 1,
        created_at: now,
        updated_at: now,
    };

    Ok(question)
}

/// Обновление вопроса.
#[server(UpdateQuestion, "/api")]
pub async fn update_question(
    payload: QuestionUpdatePayload,
) -> Result<Question, ServerFnError> {
    // TODO: Интеграция с QuestionRepository::update() из Этапа 10
    // На этапе вёрстки возвращаем мок-вопрос из payload.
    use rust_lms_shared::models::{CourseId, QuestionId, TenantId};

    let now = chrono::Utc::now();

    let question = Question {
        id: QuestionId(payload.id),
        tenant_id: TenantId::new(),
        course_id: CourseId::new(),
        title: payload.title,
        description: if payload.description.is_empty() {
            None
        } else {
            Some(payload.description)
        },
        question_type: payload.question_type,
        options: payload.options,
        correct_answer: payload.correct_answer,
        points: payload.points,
        order: payload.order,
        created_at: now,
        updated_at: now,
    };

    Ok(question)
}