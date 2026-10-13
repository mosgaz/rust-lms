// crates/client/src/admin/pages/batch_details.rs
//! Детальная страница потока обучения (Этап 8 Batches & Enrollments).
//!
//! Полное управление потоком: основная информация, участники с ролями,
//! привязанные курсы, расписание занятий, статистика прогресса.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ В `shared::BatchEnrollment` НЕТ полей `user_name`, `user_email`,
//! `progress`. При подключении БД — либо тянуть из отдельных репозиториев,
//! либо ввести DTO `EnrollmentWithUser`.
//! ⚠️ В `shared::Batch` НЕТ полей `student_count`, `course_count`. Это агрегаты.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;
use rust_lms_shared::models::{Batch, BatchEnrollment, BatchRole, BatchStatus, Course};

/// Страница деталей потока.
#[component]
pub fn AdminBatchDetails() -> impl IntoView {
    let params = use_params_map();
    // ✅ Leptos 0.8: ParamsMap::get() возвращает Option<String>
    let batch_id = move || params.with(|p| p.get("batch_id").unwrap_or_default());

    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let batch_resource = Resource::new(batch_id, |id| fetch_batch_details(id));
    let enrollments_resource = Resource::new(batch_id, |id| fetch_batch_enrollments(id));
    let courses_resource = Resource::new(batch_id, |id| fetch_batch_courses(id));
    let (active_tab, set_active_tab) = signal(BatchTab::Participants);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            <Suspense fallback=PageSkeleton>
                {move || {
                    batch_resource.get().map(|result| {
                        match result {
                            Ok(batch) => view! {
                                <BatchDetailsView
                                    batch=batch
                                    enrollments_resource=enrollments_resource
                                    courses_resource=courses_resource
                                    active_tab=active_tab.into()
                                    on_tab_change=move |tab| set_active_tab.set(tab)
                                />
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold mb-1">"Ошибка загрузки потока"</p>
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

/// Вкладки страницы потока.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BatchTab {
    /// Участники потока.
    Participants,
    /// Привязанные курсы.
    Courses,
    /// Расписание занятий.
    Schedule,
    /// Статистика.
    Statistics,
}

/// Основной вид страницы потока.
#[component]
fn BatchDetailsView(
    batch: Batch,
    // ✅ Leptos 0.8: Resource<T> без параметра source
    enrollments_resource: Resource<Result<Vec<BatchEnrollment>, ServerFnError>>,
    courses_resource: Resource<Result<Vec<Course>, ServerFnError>>,
    active_tab: Signal<BatchTab>,
    on_tab_change: impl Fn(BatchTab) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_tab_change = StoredValue::new(on_tab_change);

    let status_badge = match batch.status {
        BatchStatus::Draft => ("bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200", "Черновик"),
        BatchStatus::Active => ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "Активен"),
        BatchStatus::Archived => ("bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300", "Архив"),
        BatchStatus::Completed => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "Завершён"),
    };

    // ✅ Клонируем поля Batch до view!
    let batch_id_uuid = batch.id.0;
    let batch_title = batch.title.clone();
    let batch_clone_for_schedule = batch.clone();

    // ✅ start_date / end_date — Option<DateTime<Utc>>
    let dates_text = match (&batch.start_date, &batch.end_date) {
        (Some(s), Some(e)) => format!("{} — {}", s.format("%d.%m.%Y"), e.format("%d.%m.%Y")),
        (Some(s), None) => s.format("%d.%m.%Y").to_string(),
        (None, Some(e)) => e.format("%d.%m.%Y").to_string(),
        (None, None) => "Даты не заданы".to_string(),
    };

    // ✅ Длительность в днях (если обе даты известны)
    let duration_days: Option<i64> = match (&batch.start_date, &batch.end_date) {
        (Some(s), Some(e)) => Some((*e - *s).num_days()),
        _ => None,
    };
    let duration_text = duration_days
        .map(|d| format!("{} дней", d))
        .unwrap_or_else(|| "—".to_string());

    // ⚠️ student_count / course_count — агрегаты, которых нет в Batch
    let student_count: u32 = 0;
    let course_count: u32 = 0;

    view! {
        <div>
            // Верхняя панель
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div class="flex items-center gap-3">
                    <a
                        href="/admin/batches"
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"></path>
                        </svg>
                    </a>
                    <div>
                        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">
                            {batch_title}
                        </h1>
                        <p class="text-sm text-gray-500 dark:text-gray-400">
                            {dates_text}
                        </p>
                    </div>
                </div>

                <div class="flex items-center gap-2">
                    <span class=format!("inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {}", status_badge.0)>
                        {status_badge.1}
                    </span>
                    <button class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors">
                        "Редактировать"
                    </button>
                </div>
            </div>

            // Карточка с основной информацией
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6 mb-6">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-6">
                    <div>
                        <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">"Статус"</div>
                        <div class="text-lg font-semibold text-gray-900 dark:text-white">{status_badge.1}</div>
                    </div>
                    <div>
                        <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">"Длительность"</div>
                        <div class="text-lg font-semibold text-gray-900 dark:text-white">
                            {duration_text}
                        </div>
                    </div>
                    <div>
                        <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">"Участников"</div>
                        <div class="text-lg font-semibold text-gray-900 dark:text-white">
                            {student_count}
                        </div>
                    </div>
                    <div>
                        <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">"Курсов"</div>
                        <div class="text-lg font-semibold text-gray-900 dark:text-white">
                            {course_count}
                        </div>
                    </div>
                </div>
            </div>

            // Вкладки
            <div class="border-b border-gray-200 dark:border-gray-700 mb-6">
                <nav class="flex gap-6">
                    <TabButton
                        label="Участники"
                        active=move || active_tab.get() == BatchTab::Participants
                        on_click=move || on_tab_change.with_value(|f| f(BatchTab::Participants))
                    />
                    <TabButton
                        label="Курсы"
                        active=move || active_tab.get() == BatchTab::Courses
                        on_click=move || on_tab_change.with_value(|f| f(BatchTab::Courses))
                    />
                    <TabButton
                        label="Расписание"
                        active=move || active_tab.get() == BatchTab::Schedule
                        on_click=move || on_tab_change.with_value(|f| f(BatchTab::Schedule))
                    />
                    <TabButton
                        label="Статистика"
                        active=move || active_tab.get() == BatchTab::Statistics
                        on_click=move || on_tab_change.with_value(|f| f(BatchTab::Statistics))
                    />
                </nav>
            </div>

            // Содержимое вкладок
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                {move || {
                    match active_tab.get() {
                        BatchTab::Participants => view! {
                            <ParticipantsTab
                                batch_id=batch_id_uuid
                                enrollments_resource=enrollments_resource
                            />
                        }.into_any(),
                        BatchTab::Courses => view! {
                            <CoursesTab courses_resource=courses_resource />
                        }.into_any(),
                        BatchTab::Schedule => view! {
                            <ScheduleTab batch=batch_clone_for_schedule.clone() />
                        }.into_any(),
                        BatchTab::Statistics => view! {
                            <StatisticsTab batch_id=batch_id_uuid />
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
    // ✅ StoredValue для callbacks
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

/// Вкладка "Участники".
#[component]
fn ParticipantsTab(
    batch_id: Uuid,
    enrollments_resource: Resource<Result<Vec<BatchEnrollment>, ServerFnError>>,
) -> impl IntoView {
    // batch_id пока не используется внутри UI (только для передачи в модалку),
    // поэтому глушим warning.
    let _ = batch_id;

    let (show_add_modal, set_show_add_modal) = signal(false);
    let (search_query, set_search_query) = signal(String::new());
    let (role_filter, set_role_filter) = signal(None::<BatchRole>);

    view! {
        <div>
            // Панель действий
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-4 gap-3">
                <div class="flex gap-3 flex-1">
                    <input
                        type="text"
                        placeholder="Поиск по ID пользователя..."
                        prop:value=search_query
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))
                        class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                    />
                    <select
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            set_role_filter.set(match val.as_str() {
                                "student" => Some(BatchRole::Student),
                                "instructor" => Some(BatchRole::Instructor),
                                "tutor" => Some(BatchRole::Tutor),
                                "observer" => Some(BatchRole::Observer),
                                _ => None,
                            });
                        }
                        class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                    >
                        <option value="">"Все роли"</option>
                        <option value="student">"Студенты"</option>
                        <option value="instructor">"Инструкторы"</option>
                        <option value="tutor">"Тьюторы"</option>
                        <option value="observer">"Наблюдатели"</option>
                    </select>
                </div>
                <button
                    on:click=move |_| set_show_add_modal.set(true)
                    class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors text-sm flex items-center gap-2"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    "Добавить участника"
                </button>
            </div>

            // Таблица участников
            <Suspense fallback=TableSkeleton>
                {move || {
                    enrollments_resource.get().map(|result| {
                        match result {
                            Ok(enrollments) => {
                                let filtered = enrollments.into_iter()
                                    .filter(|e| {
                                        let q = search_query.get().to_lowercase();
                                        let matches_search = q.is_empty() ||
                                            e.user_id.to_string().to_lowercase().contains(&q);
                                        let matches_role = role_filter.get().is_none() ||
                                            e.role == role_filter.get().unwrap();
                                        matches_search && matches_role
                                    })
                                    .collect::<Vec<_>>();

                                if filtered.is_empty() {
                                    view! {
                                        <div class="text-center py-12">
                                            <p class="text-gray-500 dark:text-gray-400">"Участники не найдены"</p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="overflow-x-auto">
                                            <table class="w-full">
                                                <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                                                    <tr>
                                                        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Участник (ID)"</th>
                                                        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Роль"</th>
                                                        <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Зачислен"</th>
                                                        <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Действия"</th>
                                                    </tr>
                                                </thead>
                                                <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                                                    {filtered.into_iter().map(|enrollment| {
                                                        view! { <ParticipantRow enrollment=enrollment /> }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
                                        </div>
                                    }.into_any()
                                }
                            }
                            Err(e) => view! {
                                <div class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg text-sm">
                                    {e.to_string()}
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Модальное окно добавления участника
            <Show when=move || show_add_modal.get()>
                <AddParticipantModal
                    batch_id=batch_id
                    on_close=move || set_show_add_modal.set(false)
                    on_added=move || {
                        set_show_add_modal.set(false);
                        enrollments_resource.refetch();
                    }
                />
            </Show>
        </div>
    }
}

/// Строка таблицы участника.
///
/// ⚠️ `BatchEnrollment` не содержит `user_name`, `user_email`, `progress`.
/// Показываем `user_id` как короткий UUID. При интеграции с БД — вводим
/// DTO `EnrollmentWithUser`.
#[component]
fn ParticipantRow(enrollment: BatchEnrollment) -> impl IntoView {
    let role_badge = match enrollment.role {
        BatchRole::Student => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "Студент"),
        BatchRole::Instructor => ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "Инструктор"),
        BatchRole::Tutor => ("bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300", "Тьютор"),
        BatchRole::Observer => ("bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200", "Наблюдатель"),
    };

    let user_id_short: String = enrollment.user_id.to_string().chars().take(8).collect();
    let enrolled_str = enrollment.enrolled_at.format("%d.%m.%Y").to_string();
    let avatar_letter = role_badge.1.chars().next().unwrap_or('?').to_string();

    view! {
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
            <td class="px-4 py-3">
                <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white text-sm font-semibold">
                        {avatar_letter}
                    </div>
                    <div>
                        <div class="text-sm font-medium text-gray-900 dark:text-white font-mono">
                            {user_id_short} "..."
                        </div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">
                            "ID пользователя"
                        </div>
                    </div>
                </div>
            </td>
            <td class="px-4 py-3">
                <span class=format!("inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {}", role_badge.0)>
                    {role_badge.1}
                </span>
            </td>
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                {enrolled_str}
            </td>
            <td class="px-4 py-3 text-right">
                <div class="flex items-center justify-end gap-2">
                    <button class="text-xs text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300">
                        "Изменить роль"
                    </button>
                    <button class="text-xs text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300">
                        "Удалить"
                    </button>
                </div>
            </td>
        </tr>
    }
}

/// Модальное окно добавления участника.
#[component]
fn AddParticipantModal(
    batch_id: Uuid,
    on_close: impl Fn() + Send + Sync + 'static,
    on_added: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_added = StoredValue::new(on_added);

    let (email, set_email) = signal(String::new());
    let (role, set_role) = signal(BatchRole::Student);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let _ = add_participant_to_batch(batch_id, email.get(), role.get()).await;
            on_added.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-md w-full">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">"Добавить участника"</h3>
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
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Email пользователя *"</label>
                        <input
                            type="email"
                            prop:value=email
                            on:input=move |ev| set_email.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Роль"</label>
                        <select
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_role.set(match val.as_str() {
                                    "instructor" => BatchRole::Instructor,
                                    "tutor" => BatchRole::Tutor,
                                    "observer" => BatchRole::Observer,
                                    _ => BatchRole::Student,
                                });
                            }
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        >
                            <option value="student">"Студент"</option>
                            <option value="instructor">"Инструктор"</option>
                            <option value="tutor">"Тьютор"</option>
                            <option value="observer">"Наблюдатель"</option>
                        </select>
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700"
                        >
                            "Добавить"
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

/// Вкладка "Курсы".
#[component]
fn CoursesTab(
    courses_resource: Resource<Result<Vec<Course>, ServerFnError>>,
) -> impl IntoView {
    let (show_add_modal, set_show_add_modal) = signal(false);

    view! {
        <div>
            <div class="flex items-center justify-between mb-4">
                <p class="text-sm text-gray-600 dark:text-gray-400">"Курсы, включённые в этот поток"</p>
                <button
                    on:click=move |_| set_show_add_modal.set(true)
                    class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors text-sm flex items-center gap-2"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    "Добавить курс"
                </button>
            </div>

            <Suspense fallback=TableSkeleton>
                {move || {
                    courses_resource.get().map(|result| {
                        match result {
                            Ok(courses) => {
                                if courses.is_empty() {
                                    view! {
                                        <div class="text-center py-12">
                                            <p class="text-gray-500 dark:text-gray-400">"Курсы не добавлены"</p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-3">
                                            {courses.into_iter().map(|course| {
                                                view! { <CourseRow course=course /> }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }
                            Err(e) => view! {
                                <div class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg text-sm">
                                    {e.to_string()}
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // ⚠️ Модалка добавления курса — заготовка
            <Show when=move || show_add_modal.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-md w-full p-6">
                        <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-4">"Добавить курс"</h3>
                        <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
                            "Функция выбора курса будет доступна после интеграции с CourseRepository."
                        </p>
                        <button
                            on:click=move |_| set_show_add_modal.set(false)
                            class="w-full px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600"
                        >
                            "Закрыть"
                        </button>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Строка курса.
#[component]
fn CourseRow(course: Course) -> impl IntoView {
    let course_title = course.title.clone();
    let course_description = course.description.clone().unwrap_or_default();

    view! {
        <div class="flex items-center justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:border-blue-300 dark:hover:border-blue-600 transition-colors">
            <div class="flex-1">
                <h4 class="font-semibold text-gray-900 dark:text-white mb-1">{course_title}</h4>
                <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-1">
                    {course_description}
                </p>
            </div>
            <button class="text-sm text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300">
                "Удалить"
            </button>
        </div>
    }
}

/// Вкладка "Расписание".
#[component]
fn ScheduleTab(batch: Batch) -> impl IntoView {
    // Параметр пока не используется — глушим warning.
    let _ = batch;

    view! {
        <div>
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">"Расписание занятий и дедлайнов"</p>
            <div class="text-center py-12 text-gray-500 dark:text-gray-400">
                <p class="mb-2">"Функция расписания находится в разработке"</p>
                <p class="text-xs">"Планируется интеграция с календарём и уведомлениями"</p>
            </div>
        </div>
    }
}

/// Вкладка "Статистика".
#[component]
fn StatisticsTab(batch_id: Uuid) -> impl IntoView {
    // Параметр пока не используется — глушим warning.
    let _ = batch_id;

    view! {
        <div>
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">"Статистика прогресса потока"</p>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
                <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                    <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">"Средний прогресс"</div>
                    <div class="text-2xl font-bold text-gray-900 dark:text-white">"—"</div>
                </div>
                <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                    <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">"Завершили курс"</div>
                    <div class="text-2xl font-bold text-gray-900 dark:text-white">"—"</div>
                </div>
                <div class="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                    <div class="text-sm text-gray-600 dark:text-gray-400 mb-1">"Отстающие"</div>
                    <div class="text-2xl font-bold text-gray-900 dark:text-white">"—"</div>
                </div>
            </div>
            <div class="text-center py-8 text-gray-500 dark:text-gray-400">
                "Детальная статистика будет доступна после интеграции с аналитикой"
            </div>
        </div>
    }
}

#[component]
fn PageSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-6 animate-pulse">
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-96 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

#[component]
fn TableSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-3 animate-pulse">
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded"></div>
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded"></div>
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded"></div>
        </div>
    }
}

// ============================================================================
// Server-функции (мок-данные для вёрстки)
// ============================================================================

/// Получение деталей потока.
#[server(GetBatchDetails, "/api")]
pub async fn fetch_batch_details(_batch_id: String) -> Result<Batch, ServerFnError> {
    // TODO: Интеграция с BatchRepository::find_by_id() из Этапа 8
    // На этапе вёрстки возвращаем мок-поток.
    use chrono::{Duration, Utc};
    use rust_lms_shared::models::{BatchId, TenantId};

    let now = Utc::now();

    Ok(Batch {
        id: BatchId::new(),
        tenant_id: TenantId::new(),
        title: "Поток 2026 — Весна: Основы Rust".to_string(),
        description: Some("Групповое обучение с инструктором".to_string()),
        status: BatchStatus::Active,
        start_date: Some(now),
        end_date: Some(now + Duration::days(60)),
        enrollment_deadline: Some(now + Duration::days(14)),
    })
}

/// Получение участников потока.
#[server(GetBatchEnrollments, "/api")]
pub async fn fetch_batch_enrollments(
    _batch_id: String,
) -> Result<Vec<BatchEnrollment>, ServerFnError> {
    // TODO: Интеграция с BatchEnrollmentRepository::find_by_batch() из Этапа 8
    // На этапе вёрстки возвращаем 4 мок-участника.
    use chrono::{Duration, Utc};
    use rust_lms_shared::models::{BatchEnrollmentId, BatchId, EnrollmentStatus, UserId};

    let now = Utc::now();
    let batch_id = BatchId::new();

    let mock = vec![
        BatchEnrollment {
            id: BatchEnrollmentId::new(),
            batch_id,
            user_id: UserId::new(),
            role: BatchRole::Student,
            enrolled_at: now - Duration::days(30),
            completed_at: None,
            status: EnrollmentStatus::Active,
        },
        BatchEnrollment {
            id: BatchEnrollmentId::new(),
            batch_id,
            user_id: UserId::new(),
            role: BatchRole::Student,
            enrolled_at: now - Duration::days(25),
            completed_at: None,
            status: EnrollmentStatus::Active,
        },
        BatchEnrollment {
            id: BatchEnrollmentId::new(),
            batch_id,
            user_id: UserId::new(),
            role: BatchRole::Instructor,
            enrolled_at: now - Duration::days(40),
            completed_at: None,
            status: EnrollmentStatus::Active,
        },
        BatchEnrollment {
            id: BatchEnrollmentId::new(),
            batch_id,
            user_id: UserId::new(),
            role: BatchRole::Observer,
            enrolled_at: now - Duration::days(20),
            completed_at: None,
            status: EnrollmentStatus::Active,
        },
    ];

    Ok(mock)
}

/// Получение курсов потока.
#[server(GetBatchCourses, "/api")]
pub async fn fetch_batch_courses(_batch_id: String) -> Result<Vec<Course>, ServerFnError> {
    // TODO: JOIN batch_courses + courses из Этапа 8
    // На этапе вёрстки возвращаем 2 мок-курса.
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
            version: 1,
            completion_criteria: None,
            metadata: serde_json::json!({}),
        },
    ];

    Ok(mock)
}

/// Добавление участника в поток.
#[server(AddParticipantToBatch, "/api")]
pub async fn add_participant_to_batch(
    batch_id: Uuid,
    _email: String,
    role: BatchRole,
) -> Result<BatchEnrollment, ServerFnError> {
    // TODO: Интеграция с BatchEnrollmentRepository::enroll() из Этапа 8
    // На этапе вёрстки возвращаем мок-участника.
    use rust_lms_shared::models::{BatchEnrollmentId, BatchId, EnrollmentStatus, UserId};

    let enrollment = BatchEnrollment {
        id: BatchEnrollmentId::new(),
        batch_id: BatchId(batch_id),
        user_id: UserId::new(),
        role,
        enrolled_at: chrono::Utc::now(),
        completed_at: None,
        status: EnrollmentStatus::Active,
    };

    Ok(enrollment)
}