// crates/client/src/admin/pages/courses.rs
//! Управление курсами: список, создание, редактирование, структура (главы/уроки).
//!
//! Интеграция с Этапом 5 (Content Hierarchy) и Этапом 15 (Admin Panel).
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают заглушки.

use leptos::prelude::*;
use leptos::task::spawn_local;
use rust_lms_shared::models::Course;

/// Страница списка курсов с возможностью создания новых.
#[component]
pub fn AdminCourseList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let courses_resource = Resource::new(|| (), |_| fetch_all_courses());
    let (show_create_modal, set_show_create_modal) = signal(false);
    let (search_query, set_search_query) = signal(String::new());

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок и кнопка создания
            <div class="flex items-center justify-between mb-8">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.courses.title")
                        "Управление курсами"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.courses.subtitle")
                        "Создавайте и редактируйте учебные курсы"
                    </p>
                </div>
                <button
                    on:click=move |_| set_show_create_modal.set(true)
                    class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    // i18n: t!("admin.courses.create_button")
                    "Создать курс"
                </button>
            </div>

            // Поиск
            <div class="mb-6">
                <input
                    type="text"
                    placeholder="Поиск курсов..."
                    prop:value=search_query
                    on:input=move |ev| set_search_query.set(event_target_value(&ev))
                    class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                />
            </div>

            // Список курсов
            <Suspense fallback=CoursesSkeleton>
                {move || {
                    courses_resource.get().map(|result| {
                        match result {
                            Ok(courses) => {
                                let filtered_courses = courses.into_iter()
                                    .filter(|c| {
                                        search_query.get().is_empty() ||
                                        c.title.to_lowercase().contains(&search_query.get().to_lowercase())
                                    })
                                    .collect::<Vec<_>>();

                                if filtered_courses.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <p class="text-gray-500 dark:text-gray-400">
                                                "Курсы не найдены"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                                            {filtered_courses.into_iter().map(|course| {
                                                view! { <CourseCard course=course /> }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold">"Ошибка загрузки курсов"</p>
                                    <p class="text-sm">{e.to_string()}</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Модальное окно создания курса
            <Show when=move || show_create_modal.get()>
                <CreateCourseModal
                    on_close=move || set_show_create_modal.set(false)
                    on_created=move || {
                        set_show_create_modal.set(false);
                        courses_resource.refetch();
                    }
                />
            </Show>
        </div>
    }
}

/// Карточка курса с действиями.
#[component]
fn CourseCard(course: Course) -> impl IntoView {
    // ⚠️ В shared::Course нет поля `status` — статус не показываем.
    // При подключении БД — либо добавить `status` в DTO, либо
    // брать из CourseRepository/metadata отдельно.

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm hover:shadow-md transition-all duration-300 border border-gray-200 dark:border-gray-700 overflow-hidden flex flex-col">
            <div class="p-6 flex-grow">
                <div class="flex justify-between items-start mb-3">
                    // ⚠️ В shared::Course нет поля `status` — бейдж не показываем.
                    <span class="text-xs text-gray-500 dark:text-gray-400">
                        "v" {course.version}
                    </span>
                </div>

                <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-2 line-clamp-2">
                    {course.title.clone()}
                </h3>

                <p class="text-gray-600 dark:text-gray-300 text-sm mb-4 line-clamp-3">
                    {course.description.clone().unwrap_or_else(|| "Описание отсутствует".to_string())}
                </p>

                <div class="flex items-center gap-4 text-sm text-gray-500 dark:text-gray-400">
                    <span class="flex items-center gap-1">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                        </svg>
                        // ⚠️ В shared::Course нет `created_at` — заглушка.
                        "Дата не указана"
                    </span>
                </div>
            </div>

            <div class="px-6 py-4 bg-gray-50 dark:bg-gray-800/50 border-t border-gray-200 dark:border-gray-700 flex gap-2">
                <a
                    href=format!("/admin/courses/{}/structure", course.id)
                    class="flex-1 text-center px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                >
                    "Структура"
                </a>
                <a
                    href=format!("/admin/courses/{}/edit", course.id)
                    class="flex-1 text-center px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                >
                    "Редактировать"
                </a>
            </div>
        </div>
    }
}

/// Модальное окно создания нового курса.
#[component]
fn CreateCourseModal(
    on_close: impl Fn() + Send + Sync + 'static,
    on_created: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (Fn + Send + Sync)
    let on_close = StoredValue::new(on_close);
    let on_created = StoredValue::new(on_created);

    let (title, set_title) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_loading.set(true);

        // ✅ spawn_local из leptos::task
        spawn_local(async move {
            match create_course(title.get(), description.get()).await {
                Ok(_) => on_created.with_value(|f| f()),
                Err(e) => {
                    set_error.set(Some(e.to_string()));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-2xl font-bold text-gray-900 dark:text-white">
                        "Создать новый курс"
                    </h3>
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                <form on:submit=on_submit class="p-6 space-y-4">
                    <Show when=move || error.get().is_some()>
                        <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg text-sm">
                            {error.get().unwrap()}
                        </div>
                    </Show>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Название курса *"
                        </label>
                        <input
                            type="text"
                            prop:value=title
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Описание"
                        </label>
                        <textarea
                            prop:value=description
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                            rows="4"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                        ></textarea>
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            disabled=loading
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        >
                            {if loading.get() { "Создание..." } else { "Создать курс" }}
                        </button>
                        <button
                            type="button"
                            on:click=move |_| on_close.with_value(|f| f())
                            class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
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
fn CoursesSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
        </div>
    }
}

// ============================================================================
// Server-функции (мок-данные для вёрстки)
// ============================================================================

#[server(GetAllCourses, "/api")]
pub async fn fetch_all_courses() -> Result<Vec<Course>, ServerFnError> {
    // TODO: Интеграция с CourseRepository::find_by_tenant()
    // На этапе вёрстки возвращаем мок-курсы, чтобы страница отрисовалась.
    use rust_lms_shared::models::{CourseId, TenantId};

    let tenant_id = TenantId::new();
    let mock = vec![
        Course {
            id: CourseId::new(),
            tenant_id,
            title: "Основы Rust".to_string(),
            title_i18n: None,
            description: Some("Вводный курс по языку программирования Rust".to_string()),
            description_i18n: None,
            version: 1,
            completion_criteria: None,
            metadata: serde_json::json!({}),
        },
        Course {
            id: CourseId::new(),
            tenant_id,
            title: "Продвинутый Rust: асинхронность".to_string(),
            title_i18n: None,
            description: Some("Tokio, futures, async/await на практике".to_string()),
            description_i18n: None,
            version: 2,
            completion_criteria: None,
            metadata: serde_json::json!({}),
        },
        Course {
            id: CourseId::new(),
            tenant_id,
            title: "WebAssembly и Leptos".to_string(),
            title_i18n: None,
            description: Some("Full-stack Rust на клиенте и сервере".to_string()),
            description_i18n: None,
            version: 1,
            completion_criteria: None,
            metadata: serde_json::json!({}),
        },
    ];

    Ok(mock)
}

#[server(CreateCourse, "/api")]
pub async fn create_course(title: String, description: String) -> Result<Course, ServerFnError> {
    // TODO: Интеграция с CourseRepository::create()
    // На этапе вёрстки возвращаем мок-курс, чтобы модалка закрылась без ошибки.
    use rust_lms_shared::models::{CourseId, TenantId};

    let course = Course {
        id: CourseId::new(),
        tenant_id: TenantId::new(),
        title,
        title_i18n: None,
        description: if description.is_empty() { None } else { Some(description) },
        description_i18n: None,
        version: 1,
        completion_criteria: None,
        metadata: serde_json::json!({}),
    };

    Ok(course)
}