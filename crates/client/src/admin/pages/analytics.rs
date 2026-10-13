// crates/client/src/admin/pages/analytics.rs
//! Дашборд аналитики обучения (Этап 15.4 Admin Panel — Статистика).
//!
//! Агрегированная статистика по платформе: KPI-карточки, графики активности,
//! completion rate по курсам, топ студентов. Интеграция с данными Этапов 7–11.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Страница аналитики и статистики.
#[component]
pub fn AdminAnalyticList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let kpi_resource = Resource::new(|| (), |_| fetch_kpi_metrics());
    let activity_resource = Resource::new(|| (), |_| fetch_activity_chart());
    let completion_resource = Resource::new(|| (), |_| fetch_completion_by_course());
    let top_students_resource = Resource::new(|| (), |_| fetch_top_students());

    // Сигнал для переключения периода
    let (period, set_period) = signal(AnalyticsPeriod::Last30Days);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок и переключатель периода
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-8 gap-4">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.analytics.title")
                        "Аналитика"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.analytics.subtitle")
                        "Обзор ключевых метрик обучения"
                    </p>
                </div>

                // Переключатель периода
                <div class="flex bg-gray-100 dark:bg-gray-800 rounded-lg p-1 gap-1">
                    <PeriodButton
                        label="7 дней"
                        active=move || period.get() == AnalyticsPeriod::Last7Days
                        on_click=move || set_period.set(AnalyticsPeriod::Last7Days)
                    />
                    <PeriodButton
                        label="30 дней"
                        active=move || period.get() == AnalyticsPeriod::Last30Days
                        on_click=move || set_period.set(AnalyticsPeriod::Last30Days)
                    />
                    <PeriodButton
                        label="90 дней"
                        active=move || period.get() == AnalyticsPeriod::Last90Days
                        on_click=move || set_period.set(AnalyticsPeriod::Last90Days)
                    />
                    <PeriodButton
                        label="Всё время"
                        active=move || period.get() == AnalyticsPeriod::AllTime
                        on_click=move || set_period.set(AnalyticsPeriod::AllTime)
                    />
                </div>
            </div>

            // KPI-карточки
            <Suspense fallback=KpiSkeleton>
                {move || {
                    kpi_resource.get().map(|result| {
                        match result {
                            Ok(kpi) => view! {
                                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
                                    <KpiCard
                                        label="Всего студентов"
                                        value=kpi.total_students.to_string()
                                        change=kpi.students_change
                                        icon="👥"
                                        color="blue"
                                    />
                                    <KpiCard
                                        label="Активных (7 дн.)"
                                        value=kpi.active_students.to_string()
                                        change=kpi.active_change
                                        icon="🔥"
                                        color="green"
                                    />
                                    <KpiCard
                                        label="Завершили курс"
                                        value=kpi.completed_courses.to_string()
                                        change=kpi.completed_change
                                        icon="🎓"
                                        color="purple"
                                    />
                                    <KpiCard
                                        label="Средний прогресс"
                                        value=format!("{:.0}%", kpi.avg_progress * 100.0)
                                        change=kpi.progress_change
                                        icon="📊"
                                        color="orange"
                                    />
                                </div>
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-6 mb-8 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold">"Ошибка загрузки метрик"</p>
                                    <p class="text-sm">{e.to_string()}</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Графики
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
                // График активности
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                        // i18n: t!("admin.analytics.activity_chart_title")
                        "Активность студентов"
                    </h2>
                    <Suspense fallback=ChartSkeleton>
                        {move || {
                            activity_resource.get().map(|result| {
                                match result {
                                    Ok(data) => view! {
                                        <ActivityChart data=data />
                                    }.into_any(),
                                    Err(_) => view! {
                                        <div class="h-64 flex items-center justify-center text-gray-400 dark:text-gray-500">
                                            "Нет данных"
                                        </div>
                                    }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>

                // Completion rate по курсам
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                        // i18n: t!("admin.analytics.completion_chart_title")
                        "Завершение по курсам"
                    </h2>
                    <Suspense fallback=ChartSkeleton>
                        {move || {
                            completion_resource.get().map(|result| {
                                match result {
                                    Ok(data) => view! {
                                        <CompletionChart data=data />
                                    }.into_any(),
                                    Err(_) => view! {
                                        <div class="h-64 flex items-center justify-center text-gray-400 dark:text-gray-500">
                                            "Нет данных"
                                        </div>
                                    }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>
            </div>

            // Таблицы: Топ студентов и Отстающие
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                // Топ студентов
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                        // i18n: t!("admin.analytics.top_students_title")
                        "🏆 Лучшие студенты"
                    </h2>
                    <Suspense fallback=TableSkeleton>
                        {move || {
                            top_students_resource.get().map(|result| {
                                match result {
                                    Ok(students) => {
                                        if students.is_empty() {
                                            view! {
                                                <div class="text-center py-8 text-gray-400 dark:text-gray-500">
                                                    "Нет данных за выбранный период"
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="space-y-3">
                                                    {students.into_iter().enumerate().map(|(idx, s)| {
                                                        view! { <StudentRankRow rank=idx + 1 student=s variant="top" /> }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            }.into_any()
                                        }
                                    }
                                    Err(_) => view! { <div></div> }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>

                // Отстающие студенты
                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                        // i18n: t!("admin.analytics.at_risk_students_title")
                        "⚠️ Группа риска"
                    </h2>
                    <Suspense fallback=TableSkeleton>
                        {move || {
                            top_students_resource.get().map(|result| {
                                match result {
                                    Ok(students) => {
                                        let at_risk = students.into_iter()
                                            .filter(|s| s.avg_progress < 0.2 && s.days_inactive >= 7)
                                            .collect::<Vec<_>>();

                                        if at_risk.is_empty() {
                                            view! {
                                                <div class="text-center py-8">
                                                    <div class="text-4xl mb-2">"✅"</div>
                                                    <p class="text-gray-500 dark:text-gray-400">
                                                        // i18n: t!("admin.analytics.no_at_risk")
                                                        "Нет отстающих студентов"
                                                    </p>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="space-y-3">
                                                    {at_risk.into_iter().enumerate().map(|(idx, s)| {
                                                        view! { <StudentRankRow rank=idx + 1 student=s variant="risk" /> }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            }.into_any()
                                        }
                                    }
                                    Err(_) => view! { <div></div> }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>
            </div>
        </div>
    }
}

/// Период аналитики.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnalyticsPeriod {
    /// Последние 7 дней.
    Last7Days,
    /// Последние 30 дней.
    Last30Days,
    /// Последние 90 дней.
    Last90Days,
    /// Всё время.
    AllTime,
}

/// Кнопка переключения периода.
#[component]
fn PeriodButton(
    label: &'static str,
    // ✅ Send + Sync + 'static — требование Leptos 0.8 для замыканий в view!
    active: impl Fn() -> bool + Send + Sync + 'static,
    on_click: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue — чтобы использовать callback в Fn-контексте
    let active = StoredValue::new(active);
    let on_click = StoredValue::new(on_click);

    view! {
        <button
            on:click=move |_| on_click.with_value(|f| f())
            class=move || {
                if active.with_value(|f| f()) {
                    "px-3 py-1.5 text-sm font-medium bg-white dark:bg-gray-700 text-gray-900 dark:text-white rounded-md shadow-sm transition-all"
                } else {
                    "px-3 py-1.5 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-all"
                }
            }
        >
            {label}
        </button>
    }
}

/// KPI-карточка с индикатором изменения.
#[component]
fn KpiCard(
    label: &'static str,
    value: String,
    change: f64,
    icon: &'static str,
    color: &'static str,
) -> impl IntoView {
    let bg_color = match color {
        "blue" => "bg-blue-50 dark:bg-blue-900/20",
        "green" => "bg-green-50 dark:bg-green-900/20",
        "purple" => "bg-purple-50 dark:bg-purple-900/20",
        "orange" => "bg-orange-50 dark:bg-orange-900/20",
        _ => "bg-gray-50 dark:bg-gray-700",
    };

    let (change_color, change_arrow) = if change > 0.0 {
        ("text-green-600 dark:text-green-400", "↑")
    } else if change < 0.0 {
        ("text-red-600 dark:text-red-400", "↓")
    } else {
        ("text-gray-500 dark:text-gray-400", "→")
    };

    let change_text = format!("{:.1}%", change.abs());

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-center justify-between mb-3">
                <span class="text-sm font-medium text-gray-600 dark:text-gray-400">
                    {label}
                </span>
                <div class=format!("w-10 h-10 rounded-lg flex items-center justify-center text-xl {}", bg_color)>
                    {icon}
                </div>
            </div>
            <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
                {value}
            </div>
            <div class=format!("text-sm font-medium {}", change_color)>
                {change_arrow} " " {change_text}
                <span class="text-gray-500 dark:text-gray-400 font-normal ml-1">
                    // i18n: t!("admin.analytics.vs_previous_period")
                    "vs прошлый период"
                </span>
            </div>
        </div>
    }
}

/// График активности студентов (столбчатая диаграмма).
///
/// Использует CSS-based bar chart для минимальной зависимости.
/// При интеграции с `crates/ui/src/charts` можно заменить на полноценный компонент.
#[component]
fn ActivityChart(data: Vec<ActivityDataPoint>) -> impl IntoView {
    let max_value = data.iter().map(|d| d.active_users).max().unwrap_or(1) as f64;

    // ✅ Клонируем data — она нужна дважды: для столбцов и для подписей оси X.
    let data_for_bars = data.clone();
    let data_for_labels = data;

    view! {
        <div class="h-64 flex items-end gap-1 px-2">
            {data_for_bars.into_iter().map(|point| {
                let height_pct = (point.active_users as f64 / max_value * 100.0).max(2.0);
                view! {
                    // ✅ Каждый столбец — flex-col с h-full, чтобы процент высоты
                    // бара работал относительно высоты родителя.
                    <div class="flex-1 h-full flex flex-col justify-end items-center group relative">
                        // Tooltip
                        <div class="absolute bottom-full mb-2 px-2 py-1 bg-gray-900 dark:bg-gray-100 text-white dark:text-gray-900 text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none z-10">
                            {point.label.clone()} ": " {point.active_users}
                        </div>
                        // Столбец (высота — % от h-full родителя)
                        <div
                            class="w-full bg-blue-500 dark:bg-blue-400 rounded-t-sm hover:bg-blue-600 dark:hover:bg-blue-300 transition-all duration-300 cursor-pointer min-h-[4px]"
                            style:height=format!("{}%", height_pct)
                        ></div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
        // Подписи оси X (каждая 5-я метка)
        <div class="flex gap-1 px-2 mt-2">
            {data_for_labels.into_iter().enumerate().map(|(i, point)| {
                view! {
                    <div class="flex-1 text-center">
                        <span class=move || {
                            if i % 5 == 0 { "text-xs text-gray-500 dark:text-gray-400" } else { "text-xs text-transparent" }
                        }>
                            {point.label}
                        </span>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Горизонтальный график completion rate по курсам.
#[component]
fn CompletionChart(data: Vec<CourseCompletionData>) -> impl IntoView {
    view! {
        <div class="space-y-4">
            {data.into_iter().map(|course| {
                let pct = (course.completion_rate * 100.0).round() as u32;
                let bar_color = if pct >= 70 {
                    "bg-green-500 dark:bg-green-400"
                } else if pct >= 40 {
                    "bg-yellow-500 dark:bg-yellow-400"
                } else {
                    "bg-red-500 dark:bg-red-400"
                };

                view! {
                    <div>
                        <div class="flex items-center justify-between mb-1">
                            <span class="text-sm font-medium text-gray-900 dark:text-white truncate max-w-[200px]">
                                {course.course_title.clone()}
                            </span>
                            <span class="text-sm font-semibold text-gray-700 dark:text-gray-300 ml-2">
                                {pct} "%"
                            </span>
                        </div>
                        <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
                            <div
                                class=format!("{} h-3 rounded-full transition-all duration-700 ease-out", bar_color)
                                style:width=format!("{}%", pct)
                            ></div>
                        </div>
                        <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                            {course.completed_count} " / " {course.enrolled_count}
                            // i18n: t!("admin.analytics.students_completed")
                            " студентов"
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Строка рейтинга студента.
#[component]
fn StudentRankRow(
    rank: usize,
    student: StudentAnalytics,
    variant: &'static str,
) -> impl IntoView {
    let (rank_bg, progress_color) = if variant == "top" {
        match rank {
            1 => ("bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300", "bg-green-500"),
            2 => ("bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300", "bg-green-500"),
            3 => ("bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300", "bg-green-500"),
            _ => ("bg-gray-50 dark:bg-gray-700/50 text-gray-500 dark:text-gray-400", "bg-blue-500"),
        }
    } else {
        ("bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400", "bg-red-500")
    };

    let pct = (student.avg_progress * 100.0).round() as u32;

    // ✅ Клонируем name до view!, чтобы не словить E0382
    let name = student.name.clone();
    let avatar_letter = name.chars().next().unwrap_or('?').to_string().to_uppercase();
    let courses_completed = student.courses_completed;
    let days_inactive = student.days_inactive;

    // Предвычисляем строку "Не активен N дн." для variant="risk"
    let inactive_suffix = if variant == "risk" {
        format!(" • Не активен {} дн.", days_inactive)
    } else {
        String::new()
    };

    view! {
        <div class="flex items-center gap-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
            // Ранг
            <div class=format!("w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold {}", rank_bg)>
                {rank}
            </div>

            // Аватар
            <div class="w-10 h-10 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white font-semibold flex-shrink-0">
                {avatar_letter}
            </div>

            // Информация
            <div class="flex-1 min-w-0">
                <div class="text-sm font-medium text-gray-900 dark:text-white truncate">
                    {name}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400">
                    {courses_completed} " курсов завершено"
                    {inactive_suffix}
                </div>
            </div>

            // Прогресс
            <div class="w-20 flex-shrink-0">
                <div class="flex items-center justify-between mb-1">
                    <span class="text-xs font-semibold text-gray-700 dark:text-gray-300">
                        {pct} "%"
                    </span>
                </div>
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                    <div
                        class=format!("{} h-1.5 rounded-full", progress_color)
                        style:width=format!("{}%", pct)
                    ></div>
                </div>
            </div>
        </div>
    }
}

// Скелетоны загрузки

#[component]
fn KpiSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8 animate-pulse">
            <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

#[component]
fn ChartSkeleton() -> impl IntoView {
    view! {
        <div class="h-64 bg-gray-100 dark:bg-gray-700/50 rounded-lg animate-pulse flex items-center justify-center">
            <div class="text-gray-400 dark:text-gray-500">"Загрузка графика..."</div>
        </div>
    }
}

#[component]
fn TableSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-3 animate-pulse">
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded-lg"></div>
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded-lg"></div>
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded-lg"></div>
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded-lg"></div>
            <div class="h-16 bg-gray-100 dark:bg-gray-700/50 rounded-lg"></div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Ключевые метрики платформы.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiMetrics {
    /// Общее количество зарегистрированных студентов.
    pub total_students: usize,
    /// Изменение количества студентов относительно прошлого периода (%).
    pub students_change: f64,
    /// Количество студентов, активных за последние 7 дней.
    pub active_students: usize,
    /// Изменение активных студентов (%).
    pub active_change: f64,
    /// Количество завершённых курсов (все студенты, все курсы).
    pub completed_courses: usize,
    /// Изменение завершений (%).
    pub completed_change: f64,
    /// Средний прогресс по всем активным зачислениям (0.0–1.0).
    pub avg_progress: f64,
    /// Изменение среднего прогресса (%).
    pub progress_change: f64,
}

/// Точка данных для графика активности.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDataPoint {
    /// Подпись оси X (например, "01.10", "02.10").
    pub label: String,
    /// Количество уникальных активных пользователей в этот день.
    pub active_users: usize,
}

/// Данные о завершении конкретного курса.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseCompletionData {
    /// Название курса.
    pub course_title: String,
    /// Количество зачисленных студентов.
    pub enrolled_count: usize,
    /// Количество завершивших курс.
    pub completed_count: usize,
    /// Доля завершивших (0.0–1.0).
    pub completion_rate: f64,
}

/// Аналитика по отдельному студенту.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentAnalytics {
    /// Полное имя студента.
    pub name: String,
    /// Средний прогресс по всем курсам (0.0–1.0).
    pub avg_progress: f64,
    /// Количество завершённых курсов.
    pub courses_completed: usize,
    /// Дней с последней активности.
    pub days_inactive: usize,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

/// Получение KPI-метрик за выбранный период.
///
/// Агрегирует данные из таблиц `users`, `course_enrollments`, `lesson_progress`.
#[server(GetKpiMetrics, "/api")]
pub async fn fetch_kpi_metrics() -> Result<KpiMetrics, ServerFnError> {
    // TODO: SQL-запросы с агрегацией:
    // 1. SELECT COUNT(*) FROM users WHERE role = 'student'
    // 2. SELECT COUNT(DISTINCT user_id) FROM lesson_progress
    //    WHERE updated_at >= NOW() - INTERVAL '7 days'
    // 3. SELECT COUNT(*) FROM course_enrollments WHERE status = 'completed'
    // 4. SELECT AVG(completed_lessons_weight) FROM course_enrollments
    //    WHERE status = 'active'

    // На этапе вёрстки возвращаем мок-метрики.
    Ok(KpiMetrics {
        total_students: 1247,
        students_change: 8.3,
        active_students: 342,
        active_change: 12.5,
        completed_courses: 189,
        completed_change: -2.1,
        avg_progress: 0.47,
        progress_change: 5.6,
    })
}

/// Получение данных для графика активности.
///
/// Группировка по дням: COUNT(DISTINCT user_id) из `lesson_progress`.
#[server(GetActivityChart, "/api")]
pub async fn fetch_activity_chart() -> Result<Vec<ActivityDataPoint>, ServerFnError> {
    // TODO: SQL-запрос:
    // SELECT DATE(updated_at) as day, COUNT(DISTINCT user_id) as active_users
    // FROM lesson_progress
    // WHERE updated_at >= NOW() - INTERVAL '30 days'
    // GROUP BY DATE(updated_at)
    // ORDER BY day ASC

    // На этапе вёрстки генерируем правдоподобный паттерн за 30 дней:
    // - базовый рост от 20 до 90 пользователей;
    // - субботы и воскресенья — просадка ~40%;
    // - небольшие шумовые колебания для естественности.
    let today = chrono::Utc::now().date_naive();

    let mock: Vec<ActivityDataPoint> = (0..30)
        .map(|i| {
            // Считаем дату: 29 дней назад … сегодня
            let date = today - chrono::Duration::days((29 - i) as i64);

            // Базовый тренд: линейный рост 20 → 90
            let trend = 20.0 + (i as f64) * (70.0 / 29.0);

            // День недели: 0 = понедельник … 6 = воскресенье (chrono)
            let weekday = chrono::Datelike::weekday(&date).num_days_from_monday();
            let weekend_factor = if weekday >= 5 { 0.6 } else { 1.0 };

            // Небольшой детерминированный шум: колеблется в диапазоне ±8
            let noise = ((i as i32 * 17 + 11) % 17) as f64 - 8.0;

            let active_users = ((trend * weekend_factor) + noise).max(5.0).round() as usize;

            ActivityDataPoint {
                label: date.format("%d.%m").to_string(),
                active_users,
            }
        })
        .collect();

    Ok(mock)
}

/// Получение completion rate по каждому курсу.
///
/// JOIN `courses` + `course_enrollments` с группировкой по курсу.
#[server(GetCompletionByCourse, "/api")]
pub async fn fetch_completion_by_course() -> Result<Vec<CourseCompletionData>, ServerFnError> {
    // TODO: SQL-запрос:
    // SELECT c.title,
    //   COUNT(ce.id) as enrolled,
    //   COUNT(ce.id) FILTER (WHERE ce.status = 'completed') as completed
    // FROM courses c
    // LEFT JOIN course_enrollments ce ON ce.course_id = c.id
    // WHERE c.tenant_id = $1
    // GROUP BY c.id, c.title
    // ORDER BY enrolled DESC
    // LIMIT 10

    // На этапе вёрстки возвращаем мок-данные по 5 курсам.
    let mock = vec![
        CourseCompletionData {
            course_title: "Основы Rust".to_string(),
            enrolled_count: 250,
            completed_count: 190,
            completion_rate: 0.76,
        },
        CourseCompletionData {
            course_title: "Продвинутый Rust: асинхронность".to_string(),
            enrolled_count: 180,
            completed_count: 95,
            completion_rate: 0.53,
        },
        CourseCompletionData {
            course_title: "WebAssembly и Leptos".to_string(),
            enrolled_count: 120,
            completed_count: 38,
            completion_rate: 0.32,
        },
        CourseCompletionData {
            course_title: "SQL и PostgreSQL".to_string(),
            enrolled_count: 95,
            completed_count: 78,
            completion_rate: 0.82,
        },
        CourseCompletionData {
            course_title: "Архитектура мультитенантных систем".to_string(),
            enrolled_count: 60,
            completed_count: 22,
            completion_rate: 0.37,
        },
    ];

    Ok(mock)
}

/// Получение топа студентов по среднему прогрессу.
#[server(GetTopStudents, "/api")]
pub async fn fetch_top_students() -> Result<Vec<StudentAnalytics>, ServerFnError> {
    // TODO: SQL-запрос:
    // SELECT i.email, u.full_name,
    //   AVG(ce.completed_lessons_weight) as avg_progress,
    //   COUNT(ce.id) FILTER (WHERE ce.status = 'completed') as completed,
    //   EXTRACT(DAY FROM NOW() - MAX(lp.updated_at)) as days_inactive
    // FROM users u
    // JOIN identities i ON i.id = u.identity_id
    // JOIN course_enrollments ce ON ce.user_id = u.id
    // LEFT JOIN lesson_progress lp ON lp.user_id = u.id
    // WHERE u.tenant_id = $1
    // GROUP BY u.id, i.email, u.full_name
    // ORDER BY avg_progress DESC
    // LIMIT 20

    // На этапе вёрстки возвращаем 8 мок-студентов:
    // 5 «топовых» (avg_progress > 0.7) и 3 «в группе риска» (avg_progress < 0.2, days_inactive >= 7).
    let mock = vec![
        // Топовые
        StudentAnalytics {
            name: "Иван Студентов".to_string(),
            avg_progress: 0.95,
            courses_completed: 5,
            days_inactive: 1,
        },
        StudentAnalytics {
            name: "Мария Ученикова".to_string(),
            avg_progress: 0.88,
            courses_completed: 4,
            days_inactive: 0,
        },
        StudentAnalytics {
            name: "Алексей Прогрессов".to_string(),
            avg_progress: 0.82,
            courses_completed: 4,
            days_inactive: 2,
        },
        StudentAnalytics {
            name: "Ольга Знаниева".to_string(),
            avg_progress: 0.79,
            courses_completed: 3,
            days_inactive: 1,
        },
        StudentAnalytics {
            name: "Дмитрий Курсов".to_string(),
            avg_progress: 0.74,
            courses_completed: 3,
            days_inactive: 3,
        },
        // Группа риска
        StudentAnalytics {
            name: "Пётр Заброшенный".to_string(),
            avg_progress: 0.12,
            courses_completed: 0,
            days_inactive: 21,
        },
        StudentAnalytics {
            name: "Анна Забытая".to_string(),
            avg_progress: 0.08,
            courses_completed: 0,
            days_inactive: 14,
        },
        StudentAnalytics {
            name: "Сергей Отстающий".to_string(),
            avg_progress: 0.18,
            courses_completed: 0,
            days_inactive: 9,
        },
    ];

    Ok(mock)
}