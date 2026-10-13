// crates/client/src/admin/pages/course_edit.rs
//! Страница редактирования курса (Этап 5 + Этап 9 + Этап 15).
//!
//! Полное редактирование курса: основная информация, i18n-поля (title_i18n, description_i18n),
//! критерии завершения (Этап 9), настройки публикации, управление версиями.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_lms_shared::models::Course;

/// Страница редактирования курса.
#[component]
pub fn AdminCourseEdit() -> impl IntoView {
    let params = use_params_map();
    // ✅ Leptos 0.8: ParamsMap::get() возвращает Option<String> (владеющий)
    let course_id = move || params.with(|p| p.get("course_id").unwrap_or_default());

    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let course_resource = Resource::new(course_id, |id| fetch_course_for_edit(id));
    let (active_tab, set_active_tab) = signal(EditTab::General);
    let (save_status, set_save_status) = signal(SaveStatus::Idle);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-5xl">
            <Suspense fallback=PageSkeleton>
                {move || {
                    course_resource.get().map(|result| {
                        match result {
                            Ok(course) => view! {
                                <CourseEditForm
                                    course=course
                                    active_tab=active_tab.into()
                                    on_tab_change=move |tab| set_active_tab.set(tab)
                                    save_status=save_status.into()
                                    on_save_status_change=move |s| set_save_status.set(s)
                                />
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold mb-1">"Ошибка загрузки курса"</p>
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

/// Вкладки формы редактирования.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditTab {
    /// Основная информация и i18n.
    General,
    /// Критерии завершения (Этап 9).
    Completion,
    /// Настройки публикации и версии.
    Settings,
}

/// Статус сохранения.
///
/// ⚠️ Не `Copy` из-за `Error(String)`. Используется через `Signal<SaveStatus>`
/// и сравнение по `==`, что требует `Clone` + `PartialEq`.
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

/// Основная форма редактирования курса.
#[component]
fn CourseEditForm(
    course: Course,
    active_tab: Signal<EditTab>,
    on_tab_change: impl Fn(EditTab) + Send + Sync + 'static,
    save_status: Signal<SaveStatus>,
    on_save_status_change: impl Fn(SaveStatus) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (Fn + Send + Sync)
    let on_tab_change = StoredValue::new(on_tab_change);
    let on_save_status_change = StoredValue::new(on_save_status_change);

    // ✅ Клонируем нужные поля Course до использования в замыканиях.
    let course_id = course.id;
    let course_version = course.version;

    // Сигналы для полей формы
    let (title_ru, set_title_ru) = signal(course.title.clone());
    let title_en_initial = course
        .title_i18n
        .as_ref()
        .and_then(|v| v.get("en"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let (title_en, set_title_en) = signal(title_en_initial);

    let (desc_ru, set_desc_ru) = signal(course.description.clone().unwrap_or_default());
    let desc_en_initial = course
        .description_i18n
        .as_ref()
        .and_then(|v| v.get("en"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let (desc_en, set_desc_en) = signal(desc_en_initial);

    // Критерии завершения — в виде JSONB (см. CompletionCriteria в shared)
    let (completion_criteria, set_completion_criteria) = signal(
        course
            .completion_criteria
            .clone()
            .unwrap_or_else(|| serde_json::json!({})),
    );

    // Обработчик сохранения
    let handle_save = move || {
        on_save_status_change.with_value(|f| f(SaveStatus::Saving));

        // Собираем payload
        let title_i18n = serde_json::json!({
            "ru": title_ru.get(),
            "en": title_en.get(),
        });
        let description_i18n = serde_json::json!({
            "ru": desc_ru.get(),
            "en": desc_en.get(),
        });
        let criteria = completion_criteria.get();

        let payload = CourseUpdatePayload {
            id: course_id.0,
            title_i18n,
            description_i18n,
            completion_criteria: Some(criteria),
        };

        let on_save_status_change = on_save_status_change;
        spawn_local(async move {
            match update_course(payload).await {
                Ok(_) => on_save_status_change.with_value(|f| f(SaveStatus::Saved)),
                Err(e) => on_save_status_change
                    .with_value(|f| f(SaveStatus::Error(e.to_string()))),
            }
        });
    };

    // Обработчик публикации
    let handle_publish = move || {
        let course_id = course_id.0;
        spawn_local(async move {
            let _ = publish_course(course_id).await;
        });
    };

    view! {
        <div>
            // Верхняя панель с навигацией и действиями
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div class="flex items-center gap-3">
                    <a
                        href="/admin/courses"
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path>
                        </svg>
                    </a>
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">
                            "Редактирование курса"
                        </h1>
                        <p class="text-sm text-gray-500 dark:text-gray-400 font-mono">
                            "ID: " {course_id.to_string().chars().take(8).collect::<String>()} "..."
                        </p>
                    </div>
                </div>

                <div class="flex items-center gap-2">
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

                    // ⚠️ В shared::Course нет поля `status`. Кнопка публикации показывается всегда.
                    // При подключении БД — проверять статус через отдельный ресурс/API.
                    <button
                        on:click=move |_| handle_publish()
                        class="px-4 py-2 bg-green-600 text-white font-medium rounded-lg hover:bg-green-700 transition-colors"
                    >
                        "Опубликовать"
                    </button>
                </div>
            </div>

            // Вкладки
            <div class="border-b border-gray-200 dark:border-gray-700 mb-6">
                <nav class="flex gap-6">
                    <TabButton
                        label="Основная информация"
                        active=move || active_tab.get() == EditTab::General
                        on_click=move || on_tab_change.with_value(|f| f(EditTab::General))
                    />
                    <TabButton
                        label="Критерии завершения"
                        active=move || active_tab.get() == EditTab::Completion
                        on_click=move || on_tab_change.with_value(|f| f(EditTab::Completion))
                    />
                    <TabButton
                        label="Настройки"
                        active=move || active_tab.get() == EditTab::Settings
                        on_click=move || on_tab_change.with_value(|f| f(EditTab::Settings))
                    />
                </nav>
            </div>

            // Содержимое вкладок
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                {move || {
                    match active_tab.get() {
                        EditTab::General => view! {
                            <GeneralTab
                                title_ru=title_ru.into()
                                set_title_ru=set_title_ru
                                title_en=title_en.into()
                                set_title_en=set_title_en
                                desc_ru=desc_ru.into()
                                set_desc_ru=set_desc_ru
                                desc_en=desc_en.into()
                                set_desc_en=set_desc_en
                            />
                        }.into_any(),
                        EditTab::Completion => view! {
                            <CompletionTab
                                criteria=completion_criteria.into()
                                set_criteria=set_completion_criteria
                            />
                        }.into_any(),
                        EditTab::Settings => view! {
                            <SettingsTab course_version=course_version />
                        }.into_any(),
                    }
                }}
            </div>
        </div>
    }
}

/// Кнопка вкладки.
#[component]
fn TabButton(
    label: &'static str,
    active: impl Fn() -> bool + Send + Sync + 'static,
    on_click: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (Fn + Send + Sync)
    let active = StoredValue::new(active);
    let on_click = StoredValue::new(on_click);

    view! {
        <button
            on:click=move |_| on_click.with_value(|f| f())
            class=move || {
                if active.with_value(|f| f()) {
                    "pb-3 border-b-2 border-blue-600 text-blue-600 dark:text-blue-400 font-medium transition-colors"
                } else {
                    "pb-3 border-b-2 border-transparent text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
                }
            }
        >
            {label}
        </button>
    }
}

/// Вкладка "Основная информация" с i18n-полями.
#[component]
fn GeneralTab(
    title_ru: Signal<String>,
    set_title_ru: WriteSignal<String>,
    title_en: Signal<String>,
    set_title_en: WriteSignal<String>,
    desc_ru: Signal<String>,
    set_desc_ru: WriteSignal<String>,
    desc_en: Signal<String>,
    set_desc_en: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            // Название на русском
            <div>
                <label class="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    <span class="text-lg">"🇷🇺"</span>
                    "Название (русский) *"
                </label>
                <input
                    type="text"
                    prop:value=title_ru
                    on:input=move |ev| set_title_ru.set(event_target_value(&ev))
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                    required
                />
            </div>

            // Название на английском
            <div>
                <label class="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    <span class="text-lg">"🇬🇧"</span>
                    "Название (English)"
                </label>
                <input
                    type="text"
                    prop:value=title_en
                    on:input=move |ev| set_title_en.set(event_target_value(&ev))
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                />
            </div>

            // Описание на русском
            <div>
                <label class="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    <span class="text-lg">"🇷🇺"</span>
                    "Описание (русский)"
                </label>
                <textarea
                    prop:value=desc_ru
                    on:input=move |ev| set_desc_ru.set(event_target_value(&ev))
                    rows="4"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                    placeholder="Краткое описание курса для студентов..."
                ></textarea>
            </div>

            // Описание на английском
            <div>
                <label class="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    <span class="text-lg">"🇬🇧"</span>
                    "Описание (English)"
                </label>
                <textarea
                    prop:value=desc_en
                    on:input=move |ev| set_desc_en.set(event_target_value(&ev))
                    rows="4"
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                    placeholder="Brief course description for students..."
                ></textarea>
            </div>

            // Подсказка по i18n
            <div class="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                <div class="flex items-start gap-2">
                    <svg class="w-5 h-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <div class="text-sm text-blue-800 dark:text-blue-300">
                        <p class="font-medium mb-1">"Мультиязычность (i18n)"</p>
                        <p>
                            "Поля сохраняются в формате JSONB ("title_i18n", "description_i18n"). "
                            "Если перевод отсутствует, система использует fallback на основной язык тенанта."
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Вкладка "Критерии завершения" (Этап 9).
///
/// ⚠️ `CompletionCriteria` в shared имеет поля `mode` и `rules` (Vec<CompletionRule>),
/// а не отдельные `min_progress`/`required_nodes`/`all_lessons_completed`/`min_exam_score`.
/// На этапе вёрстки редактируем JSONB напрямую — так сохраняется совместимость с моделью.
#[component]
fn CompletionTab(
    criteria: Signal<serde_json::Value>,
    set_criteria: WriteSignal<serde_json::Value>,
) -> impl IntoView {
    // Локальный строковый сигнал для редактирования JSONB как текста.
    let criteria_text = signal({
        let initial = criteria.get_untracked();
        serde_json::to_string_pretty(&initial).unwrap_or_else(|_| "{}".to_string())
    });

    // При изменении criteria снаружи обновляем текст.
    // (На этапе вёрстки не критично, но пусть будет.)

    view! {
        <div class="space-y-6">
            <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    "Критерии завершения (JSONB)"
                </label>
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
                    "Формат: {\"mode\": \"all_of\" | \"any_of\", \"rules\": [...]}. "
                    "Подробности — CompletionCriteria в rust_lms_shared."
                </p>
                <textarea
                    rows="12"
                    prop:value=criteria_text.0
                    on:input=move |ev| {
                        let text = event_target_value(&ev);
                        criteria_text.1.set(text.clone());
                        // Пробуем распарсить и обновить criteria только если валидный JSON
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            set_criteria.set(value);
                        }
                    }
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 font-mono text-sm"
                    placeholder=r#"{"mode": "all_of", "rules": []}"#
                ></textarea>
            </div>

            <div class="p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                <div class="flex items-start gap-2">
                    <svg class="w-5 h-5 text-yellow-600 dark:text-yellow-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                    </svg>
                    <div class="text-sm text-yellow-800 dark:text-yellow-300">
                        <p class="font-medium mb-1">"В разработке"</p>
                        <p>
                            "В будущей версии здесь появится удобный визуальный редактор критериев "
                            "(min_progress, required_nodes, all_lessons_completed, min_exam_score) "
                            "с автоматической сериализацией в CompletionCriteria."
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Вкладка "Настройки".
///
/// ⚠️ В shared::Course нет полей `is_visible` и `allow_self_enrollment`.
/// На этапе вёрстки показываем только версию и опасную зону.
/// При подключении БД — либо добавить поля в DTO, либо брать из отдельного API.
#[component]
fn SettingsTab(course_version: i32) -> impl IntoView {
    view! {
        <div class="space-y-6">
            // Заглушка: Настройки видимости
            <div class="p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
                <div class="flex items-start gap-2">
                    <svg class="w-5 h-5 text-yellow-600 dark:text-yellow-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                    </svg>
                    <div class="text-sm text-yellow-800 dark:text-yellow-300">
                        <p class="font-medium mb-1">"Настройки видимости — в разработке"</p>
                        <p>
                            "В shared::Course нет полей is_visible и allow_self_enrollment. "
                            "При подключении БД — добавим в DTO или отдельный API."
                        </p>
                    </div>
                </div>
            </div>

            // Информация о версии
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 border border-gray-200 dark:border-gray-700 rounded-lg">
                <div class="flex items-center justify-between mb-2">
                    <h4 class="font-semibold text-gray-900 dark:text-white">
                        "Версия курса"
                    </h4>
                    <span class="px-2.5 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300 text-sm font-medium rounded-full">
                        "v" {course_version}
                    </span>
                </div>
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
                    "При публикации изменений создаётся новая версия курса. Студенты, уже зачисленные, продолжают обучение на старой версии."
                </p>
                <button class="px-4 py-2 bg-gray-200 dark:bg-gray-600 text-gray-700 dark:text-gray-200 text-sm font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-500 transition-colors">
                    "Создать новую версию"
                </button>
            </div>

            // Опасная зона
            <div class="p-4 border-2 border-red-200 dark:border-red-800 rounded-lg bg-red-50/50 dark:bg-red-900/10">
                <h4 class="font-semibold text-red-900 dark:text-red-300 mb-2">
                    "Опасная зона"
                </h4>
                <p class="text-sm text-red-700 dark:text-red-400 mb-3">
                    "Архивирование курса скроет его из каталога, но сохранит все данные и прогресс студентов."
                </p>
                <button class="px-4 py-2 bg-red-600 text-white text-sm font-medium rounded-lg hover:bg-red-700 transition-colors">
                    "Архивировать курс"
                </button>
            </div>
        </div>
    }
}

#[component]
fn PageSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-6 animate-pulse">
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-96 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Payload для обновления курса.
///
/// ⚠️ `completion_criteria` — `Option<serde_json::Value>`, а не `CompletionCriteria`,
/// чтобы соответствовать полю в shared::Course.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseUpdatePayload {
    /// Идентификатор курса.
    pub id: Uuid,
    /// Локализованные названия (JSONB).
    pub title_i18n: serde_json::Value,
    /// Локализованные описания (JSONB).
    pub description_i18n: serde_json::Value,
    /// Критерии завершения (JSONB).
    pub completion_criteria: Option<serde_json::Value>,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

/// Получение курса для редактирования.
#[server(GetCourseForEdit, "/api")]
pub async fn fetch_course_for_edit(_course_id: String) -> Result<Course, ServerFnError> {
    // TODO: Интеграция с CourseRepository::find_by_id() из Этапа 5
    // На этапе вёрстки возвращаем мок-курс.
    use rust_lms_shared::models::{CourseId, TenantId};

    let course = Course {
        id: CourseId::new(),
        tenant_id: TenantId::new(),
        title: "Основы Rust".to_string(),
        title_i18n: Some(serde_json::json!({
            "ru": "Основы Rust",
            "en": "Rust Basics"
        })),
        description: Some("Вводный курс по языку программирования Rust".to_string()),
        description_i18n: Some(serde_json::json!({
            "ru": "Вводный курс по языку программирования Rust",
            "en": "Introductory course on Rust programming language"
        })),
        version: 1,
        completion_criteria: Some(serde_json::json!({
            "mode": "all_of",
            "rules": []
        })),
        metadata: serde_json::json!({}),
    };

    Ok(course)
}

/// Обновление курса.
#[server(UpdateCourse, "/api")]
pub async fn update_course(payload: CourseUpdatePayload) -> Result<Course, ServerFnError> {
    // TODO: Интеграция с CourseRepository::update() + сохранение i18n-полей в JSONB
    // На этапе вёрстки возвращаем мок-курс, собранный из payload.
    use rust_lms_shared::models::{CourseId, TenantId};

    let title = payload
        .title_i18n
        .get("ru")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let description = payload
        .description_i18n
        .get("ru")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let course = Course {
        id: CourseId(payload.id),
        tenant_id: TenantId::new(),
        title,
        title_i18n: Some(payload.title_i18n),
        description,
        description_i18n: Some(payload.description_i18n),
        version: 1,
        completion_criteria: payload.completion_criteria,
        metadata: serde_json::json!({}),
    };

    Ok(course)
}

/// Публикация курса (создание новой версии).
#[server(PublishCourse, "/api")]
pub async fn publish_course(_course_id: Uuid) -> Result<Course, ServerFnError> {
    // TODO: Интеграция с CourseRepository::publish_version() из Этапа 5
    // На этапе вёрстки возвращаем мок-курс с новой версией.
    use rust_lms_shared::models::{CourseId, TenantId};

    let course = Course {
        id: CourseId(_course_id),
        tenant_id: TenantId::new(),
        title: "Основы Rust".to_string(),
        title_i18n: Some(serde_json::json!({"ru": "Основы Rust", "en": "Rust Basics"})),
        description: Some("Вводный курс".to_string()),
        description_i18n: None,
        version: 2,
        completion_criteria: None,
        metadata: serde_json::json!({}),
    };

    Ok(course)
}