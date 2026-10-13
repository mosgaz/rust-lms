// crates/client/src/admin/pages/users.rs
//! Управление пользователями (Identity-First архитектура, Этап 3).
//!
//! Список пользователей с поиском, фильтрацией по роли, управление ролями и статусами.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функция возвращает мок-данные.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Страница управления пользователями.
#[component]
pub fn AdminUserList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let users_resource = Resource::new(|| (), |_| fetch_all_users());
    let (search_query, set_search_query) = signal(String::new());
    let (role_filter, set_role_filter) = signal(None::<String>);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            <div class="flex items-center justify-between mb-8">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.users.title")
                        "Пользователи"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.users.subtitle")
                        "Управление учётными записями и ролями"
                    </p>
                </div>
            </div>

            // Фильтры
            <div class="flex gap-3 mb-6">
                <input
                    type="text"
                    placeholder="Поиск по email или имени..."
                    prop:value=search_query
                    on:input=move |ev| set_search_query.set(event_target_value(&ev))
                    class="flex-1 px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                />
                <select
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        set_role_filter.set(if val.is_empty() { None } else { Some(val) });
                    }
                    class="px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                >
                    <option value="">"Все роли"</option>
                    <option value="student">"Студенты"</option>
                    <option value="instructor">"Инструкторы"</option>
                    <option value="tutor">"Тьюторы"</option>
                    <option value="admin">"Администраторы"</option>
                </select>
            </div>

            // Таблица пользователей
            <Suspense fallback=UsersSkeleton>
                {move || {
                    users_resource.get().map(|result| {
                        match result {
                            Ok(users) => {
                                let filtered = users.into_iter()
                                    .filter(|u| {
                                        let matches_search = search_query.get().is_empty() ||
                                            u.email.to_lowercase().contains(&search_query.get().to_lowercase());
                                        let matches_role = role_filter.get().is_none() ||
                                            u.role == role_filter.get().unwrap();
                                        matches_search && matches_role
                                    })
                                    .collect::<Vec<_>>();

                                if filtered.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <p class="text-gray-500 dark:text-gray-400">
                                                // i18n: t!("admin.users.empty_state")
                                                "Пользователи не найдены"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                                            <table class="w-full">
                                                <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                                                    <tr>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Email"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Имя"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Роль"</th>
                                                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Статус"</th>
                                                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Действия"</th>
                                                    </tr>
                                                </thead>
                                                <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                                                    {filtered.into_iter().map(|user| {
                                                        view! { <UserRow user=user /> }
                                                    }).collect::<Vec<_>>()}
                                                </tbody>
                                            </table>
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
    }
}

/// Строка таблицы пользователей.
#[component]
fn UserRow(user: UserWithIdentity) -> impl IntoView {
    let role_badge = match user.role.as_str() {
        "student" => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "Студент"),
        "instructor" => ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "Инструктор"),
        "tutor" => ("bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300", "Тьютор"),
        "admin" => ("bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300", "Админ"),
        _ => ("bg-gray-100 text-gray-800", "Неизвестно"),
    };

    let status_badge = if user.is_active {
        ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "Активен")
    } else {
        ("bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300", "Неактивен")
    };

    // ✅ Клонируем значения до view!, чтобы не словить E0382
    let email = user.email.clone();
    let full_name = user
        .full_name
        .clone()
        .unwrap_or_else(|| "—".to_string());

    view! {
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-white">{email}</td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-300">
                {full_name}
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", role_badge.0)>
                    {role_badge.1}
                </span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", status_badge.0)>
                    {status_badge.1}
                </span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-right text-sm">
                <button class="text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 mr-3">
                    "Редактировать"
                </button>
                <button class="text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300">
                    "Деактивировать"
                </button>
            </td>
        </tr>
    }
}

#[component]
fn UsersSkeleton() -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden animate-pulse">
            <div class="h-12 bg-gray-200 dark:bg-gray-700"></div>
            <div class="divide-y divide-gray-200 dark:divide-gray-700">
                <div class="h-16 bg-gray-50 dark:bg-gray-800/50"></div>
                <div class="h-16 bg-gray-50 dark:bg-gray-800/50"></div>
                <div class="h-16 bg-gray-50 dark:bg-gray-800/50"></div>
                <div class="h-16 bg-gray-50 dark:bg-gray-800/50"></div>
            </div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// DTO пользователя с данными Identity (для админской таблицы).
///
/// Локальный DTO: объединяет email и full_name из Identity с ролью и статусом
/// пользователя в тенанте. Формируется на стороне сервера.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithIdentity {
    /// Email из Identity.
    pub email: String,
    /// Полное имя (если есть).
    pub full_name: Option<String>,
    /// Роль в тенанте (student/instructor/tutor/admin).
    pub role: String,
    /// Активна ли учётная запись.
    pub is_active: bool,
}

#[server(GetAllUsers, "/api")]
pub async fn fetch_all_users() -> Result<Vec<UserWithIdentity>, ServerFnError> {
    // TODO: Интеграция с UserRepository::find_by_tenant() + IdentityRepository
    // На этапе вёрстки возвращаем мок-данные.

    let mock = vec![
        UserWithIdentity {
            email: "student@example.com".to_string(),
            full_name: Some("Иван Студентов".to_string()),
            role: "student".to_string(),
            is_active: true,
        },
        UserWithIdentity {
            email: "instructor@example.com".to_string(),
            full_name: Some("Мария Преподавателева".to_string()),
            role: "instructor".to_string(),
            is_active: true,
        },
        UserWithIdentity {
            email: "tutor@example.com".to_string(),
            full_name: Some("Пётр Тьюторов".to_string()),
            role: "tutor".to_string(),
            is_active: true,
        },
        UserWithIdentity {
            email: "admin@example.com".to_string(),
            full_name: Some("Ольга Админова".to_string()),
            role: "admin".to_string(),
            is_active: true,
        },
        UserWithIdentity {
            email: "inactive@example.com".to_string(),
            full_name: None,
            role: "student".to_string(),
            is_active: false,
        },
    ];

    Ok(mock)
}