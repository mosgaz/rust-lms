// crates/client/src/admin/pages/certificates.rs
//! Просмотр и управление всеми выданными сертификатами платформы (Этап 11).
//!
//! Административная страница для суперадмина/админа тенанта:
//! - Список всех сертификатов с фильтрацией по курсу, студенту, дате выдачи
//! - Поиск по верификационному хешу и имени студента
//! - Публичная верификация и скачивание PDF
//! - Статистика выданных сертификатов
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use rust_lms_shared::models::Certificate;

/// Страница управления сертификатами (админ-панель).
#[component]
pub fn AdminCertificateList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let certificates_resource = Resource::new(|| (), |_| fetch_all_certificates());
    let stats_resource = Resource::new(|| (), |_| fetch_certificates_stats());

    // Сигналы для фильтров
    let (search_query, set_search_query) = signal(String::new());
    let (course_filter, set_course_filter) = signal(None::<String>);
    // ⚠️ Фильтрация по датам пока не реализована (TODO в блоке ниже).
    // Используем `_date_from`/`_date_to`, чтобы не было warnings об unused.
    let (_date_from, set_date_from) = signal(None::<String>);
    let (_date_to, set_date_to) = signal(None::<String>);
    let (show_filters, set_show_filters) = signal(false);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок
            <div class="flex items-center justify-between mb-8">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.certificates.title")
                        "Сертификаты"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.certificates.subtitle")
                        "Все выданные сертификаты платформы"
                    </p>
                </div>
                <button
                    on:click=move |_| set_show_filters.update(|v| *v = !*v)
                    class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors flex items-center gap-2"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"></path>
                    </svg>
                    // i18n: t!("admin.certificates.filters_toggle")
                    "Фильтры"
                </button>
            </div>

            // Карточки статистики
            <Suspense fallback=StatsSkeleton>
                {move || {
                    stats_resource.get().map(|result| {
                        match result {
                            Ok(stats) => view! {
                                <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
                                    <StatCard
                                        label="Всего выдано"
                                        value=stats.total.to_string()
                                        icon="📜"
                                        color="blue"
                                    />
                                    <StatCard
                                        label="За этот месяц"
                                        value=stats.this_month.to_string()
                                        icon="📅"
                                        color="green"
                                    />
                                    <StatCard
                                        label="Уникальных студентов"
                                        value=stats.unique_students.to_string()
                                        icon="👥"
                                        color="purple"
                                    />
                                    <StatCard
                                        label="Уникальных курсов"
                                        value=stats.unique_courses.to_string()
                                        icon="🎓"
                                        color="orange"
                                    />
                                </div>
                            }.into_any(),
                            Err(_) => view! { <div></div> }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Поиск
            <div class="mb-6">
                <div class="relative">
                    <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                    </svg>
                    <input
                        type="text"
                        placeholder="Поиск по ID студента или верификационному коду..."
                        prop:value=search_query
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))
                        class="w-full pl-10 pr-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    />
                </div>
            </div>

            // Расширенные фильтры
            <Show when=move || show_filters.get()>
                <div class="mb-6 p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg border border-gray-200 dark:border-gray-700">
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                // i18n: t!("admin.certificates.filter.course")
                                "Курс"
                            </label>
                            <select
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    set_course_filter.set(if val.is_empty() { None } else { Some(val) });
                                }
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            >
                                <option value="">"Все курсы"</option>
                                // TODO: Загрузка списка курсов из API
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                // i18n: t!("admin.certificates.filter.date_from")
                                "Дата с"
                            </label>
                            <input
                                type="date"
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    set_date_from.set(if val.is_empty() { None } else { Some(val) });
                                }
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                // i18n: t!("admin.certificates.filter.date_to")
                                "Дата по"
                            </label>
                            <input
                                type="date"
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    set_date_to.set(if val.is_empty() { None } else { Some(val) });
                                }
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            />
                        </div>
                    </div>
                    <div class="mt-4 flex justify-end">
                        <button
                            on:click=move |_| {
                                set_course_filter.set(None);
                                set_date_from.set(None);
                                set_date_to.set(None);
                                set_search_query.set(String::new());
                            }
                            class="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
                        >
                            // i18n: t!("admin.certificates.filter.reset")
                            "Сбросить фильтры"
                        </button>
                    </div>
                </div>
            </Show>

            // Таблица сертификатов
            <Suspense fallback=CertificatesSkeleton>
                {move || {
                    certificates_resource.get().map(|result| {
                        match result {
                            Ok(certificates) => {
                                let filtered = certificates.into_iter()
                                    .filter(|c| {
                                        // ⚠️ Certificate не содержит student_name/student_email/course_title.
                                        // Поиск — только по UUID student (user_id) и verification_hash.
                                        let q = search_query.get().to_lowercase();
                                        let matches_search = q.is_empty() ||
                                            c.user_id.to_string().to_lowercase().contains(&q) ||
                                            c.verification_hash.to_lowercase().contains(&q);

                                        let matches_course = course_filter.get().is_none() ||
                                            c.target_id.to_string() == course_filter.get().unwrap();

                                        // TODO: Фильтрация по датам после парсинга date_from/date_to

                                        matches_search && matches_course
                                    })
                                    .collect::<Vec<_>>();

                                // ✅ Сохраняем длину до into_iter(), чтобы избежать E0382.
                                let filtered_len = filtered.len();

                                if filtered.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <div class="text-5xl mb-4">"📜"</div>
                                            <p class="text-gray-500 dark:text-gray-400 text-lg mb-2">
                                                // i18n: t!("admin.certificates.empty_state")
                                                "Сертификаты не найдены"
                                            </p>
                                            <p class="text-gray-400 dark:text-gray-500 text-sm">
                                                // i18n: t!("admin.certificates.empty_hint")
                                                "Попробуйте изменить параметры поиска или фильтры"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                                            <div class="overflow-x-auto">
                                                <table class="w-full">
                                                    <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                                                        <tr>
                                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                                                "Студент (ID)"
                                                            </th>
                                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                                                "Цель"
                                                            </th>
                                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                                                "Дата выдачи"
                                                            </th>
                                                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                                                "Код верификации"
                                                            </th>
                                                            <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                                                "Действия"
                                                            </th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                                                        {filtered.into_iter().map(|cert| {
                                                            view! { <CertificateRow certificate=cert /> }
                                                        }).collect::<Vec<_>>()}
                                                    </tbody>
                                                </table>
                                            </div>

                                            // Пагинация (заготовка)
                                            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
                                                <div class="text-sm text-gray-600 dark:text-gray-400">
                                                    // ✅ Используем filtered_len, сохранённое до into_iter()
                                                    "Показано " {filtered_len} " сертификатов"
                                                </div>
                                                <div class="flex gap-2">
                                                    <button
                                                        disabled=true
                                                        class="px-3 py-1.5 text-sm bg-gray-100 dark:bg-gray-700 text-gray-400 dark:text-gray-500 rounded-lg cursor-not-allowed"
                                                    >
                                                        "← Назад"
                                                    </button>
                                                    <button
                                                        disabled=true
                                                        class="px-3 py-1.5 text-sm bg-gray-100 dark:bg-gray-700 text-gray-400 dark:text-gray-500 rounded-lg cursor-not-allowed"
                                                    >
                                                        "Вперёд →"
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                            }
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold mb-1">
                                        "Ошибка загрузки сертификатов"
                                    </p>
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

/// Карточка статистики.
#[component]
fn StatCard(
    label: &'static str,
    value: String,
    icon: &'static str,
    color: &'static str,
) -> impl IntoView {
    let bg_color = match color {
        "blue" => "bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400",
        "green" => "bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400",
        "purple" => "bg-purple-50 dark:bg-purple-900/20 text-purple-600 dark:text-purple-400",
        "orange" => "bg-orange-50 dark:bg-orange-900/20 text-orange-600 dark:text-orange-400",
        _ => "bg-gray-50 dark:bg-gray-700 text-gray-600 dark:text-gray-400",
    };

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
            <div class="text-3xl font-bold text-gray-900 dark:text-white">
                {value}
            </div>
        </div>
    }
}

/// Строка таблицы сертификата.
///
/// ⚠️ `Certificate` не содержит `student_name`, `student_email`, `course_title`.
/// Показываем `user_id` (короткий UUID) и `target_id` как заглушки.
/// При подключении БД — либо тянуть эти данные отдельным запросом, либо
/// использовать `CertificateView` с агрегацией на сервере.
#[component]
fn CertificateRow(certificate: Certificate) -> impl IntoView {
    let issued_date = certificate.issued_at.format("%d.%m.%Y %H:%M").to_string();
    // Показываем только первые 12 символов хеша для компактности
    let short_hash = certificate.verification_hash.chars().take(12).collect::<String>();

    // Короткие идентификаторы (первые 8 символов UUID)
    let user_id_short = certificate.user_id.to_string()[..8].to_string();
    let target_id_short = certificate.target_id.to_string()[..8].to_string();

    // Первая буква для аватара — из типа цели (К/П/Б — курс/программа/поток)
    let avatar_letter = match certificate.target_type.as_str() {
        "course" => "К",
        "program" => "П",
        "batch" => "Б",
        _ => "С",
    };

    // Человекочитаемая метка типа цели
    let target_type_label = match certificate.target_type.as_str() {
        "course" => "Курс",
        "program" => "Программа",
        "batch" => "Поток",
        _ => "Сертификат",
    };

    // Клонируем verification_hash, т.к. он нужен и в тексте, и в href
    let verification_hash_for_href = certificate.verification_hash.clone();
    let certificate_id = certificate.id;

    view! {
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
            // Студент (ID)
            <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center gap-3">
                    <div class="w-10 h-10 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white font-semibold">
                        {avatar_letter}
                    </div>
                    <div>
                        <div class="text-sm font-medium text-gray-900 dark:text-white font-mono">
                            {user_id_short} "..."
                        </div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">
                            "ID студента"
                        </div>
                    </div>
                </div>
            </td>

            // Цель
            <td class="px-6 py-4">
                <div class="text-sm text-gray-900 dark:text-white">
                    {target_type_label} " " {target_id_short} "..."
                </div>
            </td>

            // Дата выдачи
            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                {issued_date}
            </td>

            // Код верификации
            <td class="px-6 py-4 whitespace-nowrap">
                <code class="text-xs font-mono bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 px-2 py-1 rounded">
                    {short_hash} "..."
                </code>
            </td>

            // Действия
            <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                <div class="flex items-center justify-end gap-2">
                    <a
                        href=format!("/certificates/verify/{}", verification_hash_for_href)
                        target="_blank"
                        class="inline-flex items-center gap-1 px-3 py-1.5 text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition-colors"
                        title="Открыть страницу верификации"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"></path>
                        </svg>
                        "Проверить"
                    </a>
                    <a
                        href=format!("/api/v1/certificates/{}/download", certificate_id)
                        class="inline-flex items-center gap-1 px-3 py-1.5 bg-blue-600 text-white hover:bg-blue-700 rounded-lg transition-colors"
                        title="Скачать PDF"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                        </svg>
                        "PDF"
                    </a>
                </div>
            </td>
        </tr>
    }
}

/// Скелетон загрузки карточек статистики.
#[component]
fn StatsSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8 animate-pulse">
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

/// Скелетон загрузки таблицы сертификатов.
#[component]
fn CertificatesSkeleton() -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden animate-pulse">
            <div class="h-12 bg-gray-200 dark:bg-gray-700"></div>
            <div class="divide-y divide-gray-200 dark:divide-gray-700">
                <div class="h-20 bg-gray-50 dark:bg-gray-800/50"></div>
                <div class="h-20 bg-gray-50 dark:bg-gray-800/50"></div>
                <div class="h-20 bg-gray-50 dark:bg-gray-800/50"></div>
                <div class="h-20 bg-gray-50 dark:bg-gray-800/50"></div>
                <div class="h-20 bg-gray-50 dark:bg-gray-800/50"></div>
            </div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Агрегированная статистика по сертификатам.
///
/// Локальный DTO (только для админской страницы).
/// При подключении БД — считать SQL-агрегатом (см. комментарий в `fetch_certificates_stats`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificatesStats {
    /// Общее количество выданных сертификатов.
    pub total: usize,
    /// Количество сертификатов, выданных за текущий месяц.
    pub this_month: usize,
    /// Количество уникальных студентов, получивших сертификаты.
    pub unique_students: usize,
    /// Количество уникальных курсов, по которым выданы сертификаты.
    pub unique_courses: usize,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

// Бэкенд-часть (для полноценной работы):
// Добавить в CertificateRepository метод list_all_for_tenant() с пагинацией
// Добавить эндпоинт GET /api/v1/admin/certificates с поддержкой query-параметров: search, course_id, date_from, date_to, page, per_page
// Добавить эндпоинт GET /api/v1/admin/certificates/stats для агрегации

/// Получение списка всех сертификатов тенанта.
///
/// Интегрируется с `CertificateRepository::list_all_for_tenant()` из Этапа 11.
#[server(GetAllCertificates, "/api")]
pub async fn fetch_all_certificates() -> Result<Vec<Certificate>, ServerFnError> {
    // TODO: Интеграция с бэкендом:
    // let ctx = expect_context::<RlsContext>();
    // CertificateRepository::list_all_for_tenant(&ctx).await

    // На этапе вёрстки возвращаем мок-сертификаты.
    use chrono::Utc;
    use uuid::Uuid;

    let now = Utc::now();
    let mock = vec![
        Certificate {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            target_type: "course".to_string(),
            target_id: Uuid::new_v4(),
            verification_hash: "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456".to_string(),
            issued_at: now,
        },
        Certificate {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            target_type: "course".to_string(),
            target_id: Uuid::new_v4(),
            verification_hash: "b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef1234567890".to_string(),
            issued_at: now - chrono::Duration::days(5),
        },
        Certificate {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            target_type: "batch".to_string(),
            target_id: Uuid::new_v4(),
            verification_hash: "c3d4e5f6789012345678901234567890abcdef1234567890abcdef1234567890ab".to_string(),
            issued_at: now - chrono::Duration::days(30),
        },
    ];

    Ok(mock)
}

/// Получение агрегированной статистики по сертификатам.
#[server(GetCertificatesStats, "/api")]
pub async fn fetch_certificates_stats() -> Result<CertificatesStats, ServerFnError> {
    // TODO: SQL-запрос с агрегацией:
    // SELECT
    //   COUNT(*) as total,
    //   COUNT(*) FILTER (WHERE issued_at >= date_trunc('month', NOW())) as this_month,
    //   COUNT(DISTINCT user_id) as unique_students,
    //   COUNT(DISTINCT target_id) as unique_courses
    // FROM certificates WHERE tenant_id = $1

    // На этапе вёрстки возвращаем мок-статистику.
    Ok(CertificatesStats {
        total: 128,
        this_month: 23,
        unique_students: 87,
        unique_courses: 12,
    })
}