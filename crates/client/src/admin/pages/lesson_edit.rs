// crates/client/src/admin/pages/lesson_edit.rs
//! Страница редактирования урока с Markdown-редактором (Этап 5 + Этап 10 + Этап 13).
//!
//! Полное редактирование урока: контент в Markdown с превью, метаданные,
//! привязка медиафайлов (Этап 13), привязка теста (Этап 10), настройки доступа.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ В `shared::Node` НЕТ отдельных полей `content`, `duration_minutes`,
//! `is_visible`, `requires_completion`, `assessment_question_id`. Все они живут
//! в `metadata: serde_json::Value`. При чтении достаём их оттуда, при сохранении
//! — кладём обратно.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_lms_shared::models::{Node, Question};

/// Страница редактирования урока.
#[component]
pub fn AdminLessonEdit() -> impl IntoView {
    let params = use_params_map();
    // ✅ Leptos 0.8: ParamsMap::get() возвращает Option<String>
    let lesson_id = move || params.with(|p| p.get("lesson_id").unwrap_or_default());

    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let lesson_resource = Resource::new(lesson_id, |id| fetch_lesson_for_edit(id));
    let questions_resource = Resource::new(lesson_id, |id| fetch_available_questions(id));
    let (save_status, set_save_status) = signal(SaveStatus::Idle);
    let (preview_mode, set_preview_mode) = signal(false);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-6xl">
            <Suspense fallback=PageSkeleton>
                {move || {
                    lesson_resource.get().map(|result| {
                        match result {
                            Ok(lesson) => view! {
                                <LessonEditForm
                                    lesson=lesson
                                    questions_resource=questions_resource
                                    save_status=save_status.into()
                                    on_save_status_change=move |s| set_save_status.set(s)
                                    preview_mode=preview_mode.into()
                                    on_preview_toggle=move || set_preview_mode.update(|v| *v = !*v)
                                />
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold mb-1">"Ошибка загрузки урока"</p>
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
/// ⚠️ Не `Copy` из-за `Error(String)`. Используется через `Signal<SaveStatus>`.
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

/// Основная форма редактирования урока.
#[component]
fn LessonEditForm(
    lesson: Node,
    questions_resource: Resource<Result<Vec<Question>, ServerFnError>>,
    save_status: Signal<SaveStatus>,
    on_save_status_change: impl Fn(SaveStatus) + Send + Sync + 'static,
    preview_mode: Signal<bool>,
    on_preview_toggle: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_save_status_change = StoredValue::new(on_save_status_change);
    let on_preview_toggle = StoredValue::new(on_preview_toggle);

    // ✅ Клонируем поля Node до использования в замыканиях
    let lesson_id = lesson.id;
    let lesson_title = lesson.title.clone();
    let lesson_node_type_str = lesson.node_type.to_string(); // Display → String
    let lesson_course_id_str = lesson
        .course_id
        .map(|c| c.0.to_string())
        .unwrap_or_default();
    let lesson_id_short: String = lesson.id.to_string().chars().take(8).collect();

    // ✅ Достаём данные из metadata (JSONB), где они реально хранятся
    let meta = &lesson.metadata;
    let initial_content = meta
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let initial_duration = meta
        .get("duration_minutes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let initial_is_visible = meta
        .get("is_visible")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let initial_requires_completion = meta
        .get("requires_completion")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let initial_linked_question_id = meta
        .get("assessment_question_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    // Сигналы для полей формы
    let (title, set_title) = signal(lesson_title);
    let (content, set_content) = signal(initial_content);
    let (duration_minutes, set_duration_minutes) = signal(initial_duration);
    let (is_visible, set_is_visible) = signal(initial_is_visible);
    let (requires_completion, set_requires_completion) = signal(initial_requires_completion);
    let (linked_question_id, set_linked_question_id) = signal(initial_linked_question_id);

    // Обработчик сохранения
    let handle_save = move || {
        on_save_status_change.with_value(|f| f(SaveStatus::Saving));

        let payload = LessonUpdatePayload {
            id: lesson_id.0,
            title: title.get(),
            content: content.get(),
            duration_minutes: duration_minutes.get(),
            is_visible: is_visible.get(),
            requires_completion: requires_completion.get(),
            assessment_question_id: linked_question_id.get(),
        };

        let on_save_status_change = on_save_status_change;
        spawn_local(async move {
            match update_lesson(payload).await {
                Ok(_) => on_save_status_change.with_value(|f| f(SaveStatus::Saved)),
                Err(e) => on_save_status_change
                    .with_value(|f| f(SaveStatus::Error(e.to_string()))),
            }
        });
    };

    view! {
        <div>
            // Верхняя панель
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div class="flex items-center gap-3">
                    <a
                        href=format!("/admin/courses/{}/structure", lesson_course_id_str)
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path>
                        </svg>
                    </a>
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">
                            "Редактирование урока"
                        </h1>
                        <p class="text-sm text-gray-500 dark:text-gray-400">
                            {lesson_node_type_str} " • ID: " {lesson_id_short} "..."
                        </p>
                    </div>
                </div>

                <div class="flex items-center gap-2">
                    // Переключатель превью
                    <button
                        on:click=move |_| on_preview_toggle.with_value(|f| f())
                        class=move || {
                            format!(
                                "px-4 py-2 font-medium rounded-lg transition-colors {}",
                                if preview_mode.get() {
                                    "bg-blue-600 text-white hover:bg-blue-700"
                                } else {
                                    "bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600"
                                }
                            )
                        }
                    >
                        {move || if preview_mode.get() { "Редактор" } else { "Превью" }}
                    </button>

                    // Статус сохранения
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

            // Основной контент
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                // Левая колонка: редактор и метаданные
                <div class="lg:col-span-2 space-y-6">
                    // Название урока
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                            "Название урока *"
                        </label>
                        <input
                            type="text"
                            prop:value=title
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 text-lg font-semibold"
                            required
                        />
                    </div>

                    // Контент урока (Markdown)
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                        <div class="flex items-center justify-between mb-3">
                            <label class="text-sm font-medium text-gray-700 dark:text-gray-300">
                                "Содержание урока (Markdown)"
                            </label>
                            <div class="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                                </svg>
                                "Поддерживается Markdown: # заголовки, **жирный**, *курсив*"
                            </div>
                        </div>

                        <Show when=move || !preview_mode.get()>
                            // Режим редактирования
                            <textarea
                                prop:value=content
                                on:input=move |ev| set_content.set(event_target_value(&ev))
                                rows="20"
                                class="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 font-mono text-sm resize-y"
                                placeholder="# Введение&#10;&#10;Добро пожаловать на этот урок!"
                            ></textarea>
                        </Show>

                        <Show when=move || preview_mode.get()>
                            // Режим превью
                            <div class="prose prose-lg dark:prose-invert max-w-none p-4 border border-gray-200 dark:border-gray-700 rounded-lg bg-gray-50 dark:bg-gray-900/50 min-h-[400px]">
                                // TODO: Интеграция с Markdown-рендерером (pulldown-cmark)
                                <div inner_html=render_markdown_preview(&content.get())></div>
                            </div>
                        </Show>
                    </div>
                </div>

                // Правая колонка: настройки и медиа
                <div class="space-y-6">
                    // Метаданные
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                            "Метаданные"
                        </h3>
                        <div class="space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    "Длительность (минуты)"
                                </label>
                                <input
                                    type="number"
                                    min="0"
                                    prop:value=duration_minutes
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse() {
                                            set_duration_minutes.set(val);
                                        }
                                    }
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                                />
                            </div>

                            <div class="flex items-start justify-between p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
                                <div>
                                    <div class="text-sm font-medium text-gray-900 dark:text-white">
                                        "Видим для студентов"
                                    </div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                                        "Скрытые уроки не отображаются в плеере"
                                    </div>
                                </div>
                                <label class="relative inline-flex items-center cursor-pointer flex-shrink-0 ml-3">
                                    <input
                                        type="checkbox"
                                        prop:checked=is_visible
                                        on:change=move |ev| set_is_visible.set(event_target_checked(&ev))
                                        class="sr-only peer"
                                    />
                                    <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                                </label>
                            </div>

                            <div class="flex items-start justify-between p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
                                <div>
                                    <div class="text-sm font-medium text-gray-900 dark:text-white">
                                        "Обязателен для завершения"
                                    </div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                                        "Студент должен пройти урок для завершения курса"
                                    </div>
                                </div>
                                <label class="relative inline-flex items-center cursor-pointer flex-shrink-0 ml-3">
                                    <input
                                        type="checkbox"
                                        prop:checked=requires_completion
                                        on:change=move |ev| set_requires_completion.set(event_target_checked(&ev))
                                        class="sr-only peer"
                                    />
                                    <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                                </label>
                            </div>
                        </div>
                    </div>

                    // Привязка теста (Этап 10)
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                            "Тест (опционально)"
                        </h3>
                        <Suspense fallback=move || view! {
                            <div class="h-10 bg-gray-200 dark:bg-gray-700 rounded animate-pulse"></div>
                        }>
                            {move || {
                                let course_id_str = lesson_course_id_str.clone();
                                questions_resource.get().map(move |result| {
                                    match result {
                                        Ok(questions) => {
                                            if questions.is_empty() {
                                                view! {
                                                    <div class="text-center py-4">
                                                        <p class="text-sm text-gray-500 dark:text-gray-400 mb-2">
                                                            "В курсе ещё нет вопросов"
                                                        </p>
                                                        <a
                                                            href=format!("/admin/courses/{}/assessments", course_id_str)
                                                            class="text-sm text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300"
                                                        >
                                                            "Создать вопросы →"
                                                        </a>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                let current_linked = linked_question_id.get();
                                                view! {
                                                    <div>
                                                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                                            "Привязать вопрос к уроку"
                                                        </label>
                                                        <select
                                                            on:change=move |ev| {
                                                                let val = event_target_value(&ev);
                                                                let question_id = if val.is_empty() {
                                                                    None
                                                                } else {
                                                                    Uuid::parse_str(&val).ok()
                                                                };
                                                                set_linked_question_id.set(question_id);
                                                            }
                                                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                                                        >
                                                            <option value="" selected=current_linked.is_none()>
                                                                "— Без теста —"
                                                            </option>
                                                            {questions.into_iter().map(|q| {
                                                                // ✅ Извлекаем внутренний Uuid из newtype QuestionId
                                                                let q_id = q.id.0;
                                                                let q_title = q.title.clone();
                                                                let is_selected = current_linked == Some(q_id);
                                                                view! {
                                                                    <option
                                                                        value=q_id.to_string()
                                                                        selected=is_selected
                                                                    >
                                                                        {q_title}
                                                                    </option>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </select>
                                                        <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">
                                                            "Студент должен успешно пройти тест для завершения урока"
                                                        </p>
                                                    </div>
                                                }.into_any()
                                            }
                                        }
                                        Err(_) => view! {
                                            <div class="text-sm text-red-600 dark:text-red-400">
                                                "Ошибка загрузки вопросов"
                                            </div>
                                        }.into_any(),
                                    }
                                })
                            }}
                        </Suspense>
                    </div>

                    // Медиафайлы (Этап 13)
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                            "Медиафайлы"
                        </h3>
                        <div class="border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-6 text-center hover:border-blue-400 dark:hover:border-blue-500 transition-colors cursor-pointer">
                            <svg class="w-12 h-12 mx-auto text-gray-400 dark:text-gray-500 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
                            </svg>
                            <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">
                                "Перетащите файлы сюда или нажмите для загрузки"
                            </p>
                            <p class="text-xs text-gray-500 dark:text-gray-500">
                                "JPG, PNG, MP4, PDF до 100MB"
                            </p>
                            <button class="mt-3 px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors">
                                "Выбрать файлы"
                            </button>
                        </div>
                        <p class="text-xs text-gray-500 dark:text-gray-400 mt-3">
                            "После загрузки используйте URL файла в Markdown: ![alt](url)"
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Временная функция для рендеринга Markdown в HTML.
///
/// TODO: Заменить на полноценный Markdown-парсер (pulldown-cmark).
fn render_markdown_preview(markdown: &str) -> String {
    // Простейшая замена для демонстрации
    markdown
        .replace("# ", "<h1>")
        .replace("## ", "<h2>")
        .replace("### ", "<h3>")
        .replace("**", "<strong>")
        .replace("*", "<em>")
        .replace("\n\n", "</p><p>")
}

#[component]
fn PageSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-6 animate-pulse">
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <div class="lg:col-span-2 space-y-6">
                    <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
                    <div class="h-96 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
                </div>
                <div class="space-y-6">
                    <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
                    <div class="h-48 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
                    <div class="h-48 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Payload для обновления урока.
///
/// ⚠️ `content`, `duration_minutes`, `is_visible`, `requires_completion`,
/// `assessment_question_id` при сохранении кладутся в `metadata` (JSONB),
/// т.к. в `shared::Node` нет отдельных полей для них.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonUpdatePayload {
    /// ID урока.
    pub id: Uuid,
    /// Название урока.
    pub title: String,
    /// Markdown-контент урока.
    pub content: String,
    /// Длительность в минутах.
    pub duration_minutes: u32,
    /// Видим ли урок студентам.
    pub is_visible: bool,
    /// Обязателен ли для завершения курса.
    pub requires_completion: bool,
    /// ID привязанного вопроса (если есть).
    pub assessment_question_id: Option<Uuid>,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

/// Получение урока для редактирования.
#[server(GetLessonForEdit, "/api")]
pub async fn fetch_lesson_for_edit(_lesson_id: String) -> Result<Node, ServerFnError> {
    // TODO: Интеграция с NodeRepository::find_by_id() из Этапа 5
    // На этапе вёрстки возвращаем мок-урок с наполненным metadata.
    use rust_lms_shared::models::{CourseId, NodeId, NodeType, TenantId};

    let node = Node {
        id: NodeId::new(),
        tenant_id: TenantId::new(),
        parent_id: None,
        node_type: NodeType::Lesson,
        course_id: Some(CourseId::new()),
        title: "Урок 1.2. Первая программа".to_string(),
        title_i18n: None,
        description: Some("Пишем Hello, world!".to_string()),
        metadata: serde_json::json!({
            "content": "# Первая программа на Rust\n\nНапишем классический `Hello, world!`.\n\n## Структура проекта\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```",
            "duration_minutes": 20,
            "is_visible": true,
            "requires_completion": false,
            "assessment_question_id": null,
        }),
        sort_order: 1,
        is_archived: false,
        archived_at: None,
    };

    Ok(node)
}

/// Получение списка вопросов курса для привязки к уроку.
#[server(GetAvailableQuestions, "/api")]
pub async fn fetch_available_questions(
    _lesson_id: String,
) -> Result<Vec<Question>, ServerFnError> {
    // TODO: Интеграция с QuestionRepository::list_by_course() из Этапа 10
    // На этапе вёрстки возвращаем 2 мок-вопроса.
    use rust_lms_shared::models::{CourseId, QuestionId, QuestionType, TenantId};

    let now = chrono::Utc::now();
    let tenant_id = TenantId::new();
    let course_id = CourseId::new();

    let mock = vec![
        Question {
            id: QuestionId::new(),
            tenant_id,
            course_id,
            title: "Какой инструмент управляет версиями Rust?".to_string(),
            description: None,
            question_type: QuestionType::MultipleChoice,
            options: None,
            correct_answer: Some("rustup".to_string()),
            points: 1,
            order: 1,
            created_at: now,
            updated_at: now,
        },
        Question {
            id: QuestionId::new(),
            tenant_id,
            course_id,
            title: "Rust использует GC для управления памятью?".to_string(),
            description: None,
            question_type: QuestionType::TrueFalse,
            options: None,
            correct_answer: Some("false".to_string()),
            points: 1,
            order: 2,
            created_at: now,
            updated_at: now,
        },
    ];

    Ok(mock)
}

/// Обновление урока.
#[server(UpdateLesson, "/api")]
pub async fn update_lesson(payload: LessonUpdatePayload) -> Result<Node, ServerFnError> {
    // TODO: Интеграция с NodeRepository::update() из Этапа 5
    // На этапе вёрстки возвращаем мок-урок из payload,
    // собирая metadata из переданных полей.

    use rust_lms_shared::models::{CourseId, NodeId, NodeType, TenantId};

    // Собираем metadata из payload
    let mut metadata = serde_json::Map::new();
    metadata.insert("content".to_string(), serde_json::Value::String(payload.content));
    metadata.insert(
        "duration_minutes".to_string(),
        serde_json::Value::Number(payload.duration_minutes.into()),
    );
    metadata.insert(
        "is_visible".to_string(),
        serde_json::Value::Bool(payload.is_visible),
    );
    metadata.insert(
        "requires_completion".to_string(),
        serde_json::Value::Bool(payload.requires_completion),
    );
    if let Some(qid) = payload.assessment_question_id {
        metadata.insert(
            "assessment_question_id".to_string(),
            serde_json::Value::String(qid.to_string()),
        );
    }

    let node = Node {
        id: NodeId(payload.id),
        tenant_id: TenantId::new(),
        parent_id: None,
        node_type: NodeType::Lesson,
        course_id: Some(CourseId::new()),
        title: payload.title,
        title_i18n: None,
        description: None,
        metadata: serde_json::Value::Object(metadata),
        sort_order: 0,
        is_archived: false,
        archived_at: None,
    };

    Ok(node)
}