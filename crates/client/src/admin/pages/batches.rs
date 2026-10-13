// crates/client/src/admin/pages/batches.rs
//! Управление потоками обучения (когортное обучение, Этап 8).
//!
//! Создание потоков, привязка курсов, управление участниками и ролями.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.

use leptos::prelude::*;
use leptos::task::spawn_local;
use rust_lms_shared::models::{Batch, BatchStatus};

/// Страница управления потоками.
#[component]
pub fn AdminBatchList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let batches_resource = Resource::new(|| (), |_| fetch_all_batches());
    let (show_create_modal, set_show_create_modal) = signal(false);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            <div class="flex items-center justify-between mb-8">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.batches.title")
                        "Потоки обучения"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.batches.subtitle")
                        "Когортное обучение с фиксированным расписанием"
                    </p>
                </div>
                <button
                    on:click=move |_| set_show_create_modal.set(true)
                    class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    // i18n: t!("admin.batches.create_button")
                    "Создать поток"
                </button>
            </div>

            <Suspense fallback=BatchesSkeleton>
                {move || {
                    batches_resource.get().map(|result| {
                        match result {
                            Ok(batches) => {
                                if batches.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <p class="text-gray-500 dark:text-gray-400">
                                                // i18n: t!("admin.batches.empty_state")
                                                "Потоки ещё не созданы"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                            {batches.into_iter().map(|batch| {
                                                view! { <BatchCard batch=batch /> }
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

            <Show when=move || show_create_modal.get()>
                <CreateBatchModal
                    on_close=move || set_show_create_modal.set(false)
                    on_created=move || {
                        set_show_create_modal.set(false);
                        batches_resource.refetch();
                    }
                />
            </Show>
        </div>
    }
}

/// Карточка потока.
#[component]
fn BatchCard(batch: Batch) -> impl IntoView {
    let status_badge = match batch.status {
        BatchStatus::Draft => ("bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200", "Черновик"),
        BatchStatus::Active => ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "Активен"),
        BatchStatus::Archived => ("bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300", "Архив"),
        BatchStatus::Completed => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "Завершён"),
    };

    // ✅ Клонируем поля до view!, чтобы избежать E0382
    let title = batch.title.clone();
    let batch_id = batch.id.0;

    // ✅ start_date и end_date — Option<DateTime<Utc>>
    // Форматируем в Option<String>, чтобы UI показал заглушку, если даты нет.
    let dates_text = match (&batch.start_date, &batch.end_date) {
        (Some(start), Some(end)) => {
            format!("{} — {}", start.format("%d.%m.%Y"), end.format("%d.%m.%Y"))
        }
        (Some(start), None) => start.format("%d.%m.%Y").to_string(),
        (None, Some(end)) => end.format("%d.%m.%Y").to_string(),
        (None, None) => "Даты не заданы".to_string(),
    };

    // ⚠️ В shared::Batch нет полей `student_count` и `course_count`.
    // До подключения БД показываем заглушки (0). При интеграции — тянуть
    // из BatchEnrollmentRepository / batch_courses или расширить DTO.
    let student_count: u32 = 0;
    let course_count: u32 = 0;

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm hover:shadow-md transition-all duration-300 border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-start justify-between mb-4">
                <div>
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-2">{title}</h3>
                    <p class="text-sm text-gray-600 dark:text-gray-400">
                        {dates_text}
                    </p>
                </div>
                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", status_badge.0)>
                    {status_badge.1}
                </span>
            </div>

            <div class="grid grid-cols-2 gap-4 mb-6 text-sm">
                <div>
                    <div class="text-gray-600 dark:text-gray-400 mb-1">
                        // i18n: t!("admin.batches.participants")
                        "Участники"
                    </div>
                    <div class="text-2xl font-bold text-gray-900 dark:text-white">{student_count}</div>
                </div>
                <div>
                    <div class="text-gray-600 dark:text-gray-400 mb-1">
                        // i18n: t!("admin.batches.courses")
                        "Курсов"
                    </div>
                    <div class="text-2xl font-bold text-gray-900 dark:text-white">{course_count}</div>
                </div>
            </div>

            <div class="flex gap-2">
                <a
                    href=format!("/admin/batches/{}/details", batch_id)
                    class="flex-1 text-center px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                >
                    "Подробнее"
                </a>
                <button class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors">
                    "Редактировать"
                </button>
            </div>
        </div>
    }
}

/// Модальное окно создания потока.
#[component]
fn CreateBatchModal(
    on_close: impl Fn() + Send + Sync + 'static,
    on_created: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (Fn + Send + Sync)
    let on_close = StoredValue::new(on_close);
    let on_created = StoredValue::new(on_created);

    let (name, set_name) = signal(String::new());
    let (start_date, set_start_date) = signal(String::new());
    let (end_date, set_end_date) = signal(String::new());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        // ✅ spawn_local из leptos::task
        spawn_local(async move {
            let _ = create_batch(name.get(), start_date.get(), end_date.get()).await;
            on_created.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-md w-full">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-2xl font-bold text-gray-900 dark:text-white">
                        "Создать поток"
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
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Название потока *"
                        </label>
                        <input
                            type="text"
                            prop:value=name
                            on:input=move |ev| set_name.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Дата начала *"
                        </label>
                        <input
                            type="date"
                            prop:value=start_date
                            on:input=move |ev| set_start_date.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Дата окончания *"
                        </label>
                        <input
                            type="date"
                            prop:value=end_date
                            on:input=move |ev| set_end_date.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            required
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
fn BatchesSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
        </div>
    }
}

// ============================================================================
// Server-функции (мок-данные для вёрстки)
// ============================================================================

#[server(GetAllBatches, "/api")]
pub async fn fetch_all_batches() -> Result<Vec<Batch>, ServerFnError> {
    // TODO: Интеграция с BatchRepository::find_by_tenant()
    // На этапе вёрстки возвращаем мок-данные.
    use chrono::{Duration, Utc};
    use rust_lms_shared::models::{BatchId, TenantId};

    let tenant_id = TenantId::new();
    let now = Utc::now();

    let mock = vec![
        Batch {
            id: BatchId::new(),
            tenant_id,
            title: "Поток 2026 — Весна: Основы Rust".to_string(),
            description: Some("Групповое обучение с инструктором".to_string()),
            status: BatchStatus::Active,
            start_date: Some(now),
            end_date: Some(now + Duration::days(60)),
            enrollment_deadline: Some(now + Duration::days(14)),
        },
        Batch {
            id: BatchId::new(),
            tenant_id,
            title: "Поток 2026 — Лето: Продвинутый Rust".to_string(),
            description: None,
            status: BatchStatus::Draft,
            start_date: Some(now + Duration::days(90)),
            end_date: Some(now + Duration::days(150)),
            enrollment_deadline: None,
        },
    ];

    Ok(mock)
}

#[server(CreateBatch, "/api")]
pub async fn create_batch(
    name: String,
    start_date: String,
    end_date: String,
) -> Result<Batch, ServerFnError> {
    // TODO: Интеграция с BatchRepository::create()
    // На этапе вёрстки возвращаем мок-поток.

    use chrono::{DateTime, NaiveDate, Utc};
    use rust_lms_shared::models::{BatchId, TenantId};

    // Парсим даты из строк "YYYY-MM-DD" в DateTime<Utc>.
    // Если парсинг не удался — None.
    let parse_date = |s: &str| -> Option<DateTime<Utc>> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    };

    let batch = Batch {
        id: BatchId::new(),
        tenant_id: TenantId::new(),
        title: name,
        description: None,
        status: BatchStatus::Draft,
        start_date: parse_date(&start_date),
        end_date: parse_date(&end_date),
        enrollment_deadline: None,
    };

    Ok(batch)
}