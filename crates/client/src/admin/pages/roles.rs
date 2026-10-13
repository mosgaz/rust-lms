// crates/client/src/admin/pages/roles.rs
//! Управление ролями пользователей (объединённая страница для instructor/mentor).
//!
//! CRUD для ролей в системе: предопределённые (student, instructor, tutor, observer, admin)
//! и кастомные роли. Назначение ролей пользователям, матрица разрешений (permissions),
//! RBAC-интеграция с Identity-First архитектурой (Этап 3).
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ `Role`, `Permission`, `UserRoleAssignment` — локальные DTO:
//! они ещё не вынесены в `rust_lms_shared::models`. При интеграции с Этапом 3
//! — перенести в shared (по аналогии с Course/Node/Certificate).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Страница управления ролями.
#[component]
pub fn AdminRoleList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let roles_resource = Resource::new(|| (), |_| fetch_all_roles());
    let assignments_resource = Resource::new(|| (), |_| fetch_role_assignments());

    // Сигналы состояния
    let (active_tab, set_active_tab) = signal(RolesTab::Roles);
    let (show_create_modal, set_show_create_modal) = signal(false);
    let (editing_role, set_editing_role) = signal(None::<Role>);
    let (selected_role, set_selected_role) = signal(None::<Role>);
    let (toast, set_toast) = signal(None::<ToastMessage>);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
                        <span class="text-2xl">"👥"</span>
                        // i18n: t!("admin.roles.title")
                        "Управление ролями"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.roles.subtitle")
                        "Настройка ролей и разрешений пользователей"
                    </p>
                </div>

                <button
                    on:click=move |_| set_show_create_modal.set(true)
                    class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    "Создать роль"
                </button>
            </div>

            // Информационный баннер
            <div class="mb-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                <div class="flex items-start gap-3">
                    <svg class="w-5 h-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <div class="text-sm text-blue-800 dark:text-blue-300">
                        <p class="font-medium mb-1">"Identity-First архитектура"</p>
                        <p>
                            "Роли назначаются в контексте тенанта. Одна личность (Identity) может иметь "
                            "разные роли в разных организациях. Предопределённые роли нельзя удалить."
                        </p>
                    </div>
                </div>
            </div>

            // Статистика
            <Suspense fallback=StatsSkeleton>
                {move || {
                    roles_resource.get().map(|result| {
                        match result {
                            Ok(roles) => {
                                let total = roles.len();
                                let custom = roles.iter().filter(|r| !r.is_system).count();
                                let assignments_count = roles.iter().map(|r| r.user_count).sum::<usize>();

                                view! {
                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Всего ролей"</div>
                                            <div class="text-2xl font-bold text-gray-900 dark:text-white">{total}</div>
                                        </div>
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Кастомных"</div>
                                            <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{custom}</div>
                                        </div>
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Назначений"</div>
                                            <div class="text-2xl font-bold text-purple-600 dark:text-purple-400">{assignments_count}</div>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            Err(_) => view! { <div></div> }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Вкладки
            <div class="border-b border-gray-200 dark:border-gray-700 mb-6">
                <nav class="flex gap-6">
                    <TabButton
                        label="Роли"
                        active=move || active_tab.get() == RolesTab::Roles
                        on_click=move || set_active_tab.set(RolesTab::Roles)
                    />
                    <TabButton
                        label="Назначения"
                        active=move || active_tab.get() == RolesTab::Assignments
                        on_click=move || set_active_tab.set(RolesTab::Assignments)
                    />
                    <TabButton
                        label="Разрешения"
                        active=move || active_tab.get() == RolesTab::Permissions
                        on_click=move || set_active_tab.set(RolesTab::Permissions)
                    />
                </nav>
            </div>

            // Содержимое вкладок
            {move || {
                match active_tab.get() {
                    RolesTab::Roles => view! {
                        <RolesListTab
                            roles_resource=roles_resource
                            on_edit=move |role| set_editing_role.set(Some(role))
                            on_view=move |role| set_selected_role.set(Some(role))
                        />
                    }.into_any(),
                    RolesTab::Assignments => view! {
                        <AssignmentsTab assignments_resource=assignments_resource />
                    }.into_any(),
                    RolesTab::Permissions => view! {
                        <PermissionsTab roles_resource=roles_resource />
                    }.into_any(),
                }
            }}

            // Модалка создания/редактирования
            <Show when=move || show_create_modal.get() || editing_role.get().is_some()>
                <RoleModal
                    role=editing_role.get()
                    on_close=move || {
                        set_show_create_modal.set(false);
                        set_editing_role.set(None);
                    }
                    on_saved=move || {
                        set_show_create_modal.set(false);
                        set_editing_role.set(None);
                        roles_resource.refetch();
                        set_toast.set(Some(ToastMessage {
                            message: "Роль сохранена".to_string(),
                            variant: ToastVariant::Success,
                        }));
                    }
                />
            </Show>

            // Модалка деталей роли
            <Show when=move || selected_role.get().is_some()>
                <RoleDetailModal
                    role=selected_role.get().unwrap()
                    on_close=move || set_selected_role.set(None)
                />
            </Show>

            // Toast
            <Show when=move || toast.get().is_some()>
                <Toast
                    message=toast.get().unwrap()
                    on_close=move || set_toast.set(None)
                />
            </Show>
        </div>
    }
}

/// Вкладки страницы ролей.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RolesTab {
    /// Список ролей.
    Roles,
    /// Назначения пользователям.
    Assignments,
    /// Матрица разрешений.
    Permissions,
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

/// Вкладка "Список ролей".
#[component]
fn RolesListTab(
    // ✅ Leptos 0.8: Resource<T> без параметра source
    roles_resource: Resource<Result<Vec<Role>, ServerFnError>>,
    on_edit: impl Fn(Role) + Send + Sync + 'static,
    on_view: impl Fn(Role) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_edit = StoredValue::new(on_edit);
    let on_view = StoredValue::new(on_view);

    view! {
        <Suspense fallback=TableSkeleton>
            {move || {
                roles_resource.get().map(|result| {
                    match result {
                        Ok(roles) => {
                            if roles.is_empty() {
                                view! {
                                    <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                        <div class="text-5xl mb-4">"👥"</div>
                                        <p class="text-gray-500 dark:text-gray-400 text-lg mb-1">"Роли не найдены"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                        {roles.into_iter().map(|role| {
                                            let role_for_edit = role.clone();
                                            let role_for_view = role.clone();
                                            view! {
                                                <RoleCard
                                                    role=role
                                                    on_edit=move || on_edit.with_value(|f| f(role_for_edit.clone()))
                                                    on_view=move || on_view.with_value(|f| f(role_for_view.clone()))
                                                />
                                            }
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
    }
}

/// Карточка роли.
#[component]
fn RoleCard(
    role: Role,
    on_edit: impl Fn() + Send + Sync + 'static,
    on_view: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_edit = StoredValue::new(on_edit);
    let on_view = StoredValue::new(on_view);

    let icon = get_role_icon(&role.key);
    let color = get_role_color(&role.key);
    let is_system = role.is_system;
    let name = role.name.clone();
    let description = role.description.clone();
    let user_count = role.user_count;
    let permissions_count = role.permissions_count;

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-5 hover:shadow-md transition-all">
            <div class="flex items-start justify-between mb-4">
                <div class=format!("w-12 h-12 rounded-xl flex items-center justify-center text-2xl {}", color)>
                    {icon}
                </div>
                <Show when=move || is_system>
                    <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300">
                        "🔒 Системная"
                    </span>
                </Show>
            </div>

            <h3 class="text-lg font-bold text-gray-900 dark:text-white mb-1">
                {name}
            </h3>
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-4 line-clamp-2">
                {description}
            </p>

            <div class="flex items-center justify-between text-sm mb-4">
                <span class="text-gray-500 dark:text-gray-400">
                    "👥 " {user_count} " пользователей"
                </span>
                <span class="text-gray-500 dark:text-gray-400">
                    "🔐 " {permissions_count} " разрешений"
                </span>
            </div>

            <div class="flex gap-2">
                <button
                    on:click=move |_| on_view.with_value(|f| f())
                    class="flex-1 px-3 py-1.5 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 text-sm font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                >
                    "Подробнее"
                </button>
                <Show when=move || !is_system>
                    <button
                        on:click=move |_| on_edit.with_value(|f| f())
                        class="flex-1 px-3 py-1.5 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors"
                    >
                        "Изменить"
                    </button>
                </Show>
            </div>
        </div>
    }
}

/// Вкладка "Назначения ролей".
#[component]
fn AssignmentsTab(
    assignments_resource: Resource<Result<Vec<UserRoleAssignment>, ServerFnError>>,
) -> impl IntoView {
    let (search_query, set_search_query) = signal(String::new());
    let (_role_filter, _set_role_filter) = signal(None::<String>);

    view! {
        <div>
            <div class="flex flex-col sm:flex-row gap-3 mb-4">
                <input
                    type="text"
                    placeholder="Поиск по имени или email..."
                    prop:value=search_query
                    on:input=move |ev| set_search_query.set(event_target_value(&ev))
                    class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                />
                <button class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors text-sm">
                    "Назначить роль"
                </button>
            </div>

            <Suspense fallback=TableSkeleton>
                {move || {
                    assignments_resource.get().map(|result| {
                        match result {
                            Ok(assignments) => {
                                if assignments.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <p class="text-gray-500 dark:text-gray-400">"Назначения не найдены"</p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                                            <div class="overflow-x-auto">
                                                <table class="w-full">
                                                    <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                                                        <tr>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Пользователь"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Роль"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Тенант"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Назначена"</th>
                                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Действия"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                                                        {assignments.into_iter().map(|a| {
                                                            view! { <AssignmentRow assignment=a /> }
                                                        }).collect::<Vec<_>>()}
                                                    </tbody>
                                                </table>
                                            </div>
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

/// Строка таблицы назначений.
#[component]
fn AssignmentRow(assignment: UserRoleAssignment) -> impl IntoView {
    // ✅ Клонируем поля до view! — предотвращает E0382
    let user_name = assignment.user_name.clone();
    let user_email = assignment.user_email.clone();
    let role_name = assignment.role_name.clone();
    let role_icon = get_role_icon(&assignment.role_key);
    let tenant_name = assignment.tenant_name.clone();
    // ✅ DelayedFormat → String до view! (DelayedFormat не RenderHtml)
    let assigned_at_str = assignment.assigned_at.format("%d.%m.%Y").to_string();
    let avatar_letter = user_name.chars().next().unwrap_or('?').to_string().to_uppercase();

    view! {
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors">
            <td class="px-4 py-3">
                <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-full bg-gradient-to-br from-blue-400 to-purple-500 flex items-center justify-center text-white text-xs font-bold">
                        {avatar_letter}
                    </div>
                    <div>
                        <div class="text-sm font-medium text-gray-900 dark:text-white">{user_name}</div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">{user_email}</div>
                    </div>
                </div>
            </td>
            <td class="px-4 py-3">
                <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300">
                    {role_icon} " " {role_name}
                </span>
            </td>
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">{tenant_name}</td>
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">{assigned_at_str}</td>
            <td class="px-4 py-3 text-right">
                <button class="text-xs text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300">"Отозвать"</button>
            </td>
        </tr>
    }
}

/// Вкладка "Матрица разрешений".
#[component]
fn PermissionsTab(
    roles_resource: Resource<Result<Vec<Role>, ServerFnError>>,
) -> impl IntoView {
    let all_permissions = get_all_permissions();

    view! {
        <Suspense fallback=TableSkeleton>
            {move || {
                let permissions = all_permissions.clone();
                roles_resource.get().map(move |result| {
                    match result {
                        Ok(roles) => {
                            let roles_for_body = roles.clone();
                            view! {
                                <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                                    <div class="overflow-x-auto">
                                        <table class="w-full">
                                            <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                                                <tr>
                                                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase sticky left-0 bg-gray-50 dark:bg-gray-700/50">"Разрешение"</th>
                                                    {roles.iter().map(|role| {
                                                        let role_name = role.name.clone();
                                                        let role_name_for_title = role_name.clone();
                                                        view! {
                                                            <th class="px-3 py-3 text-center text-xs font-medium text-gray-500 dark:text-gray-400 uppercase min-w-[100px]">
                                                                <div class="truncate" title=role_name_for_title>{role_name}</div>
                                                            </th>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </tr>
                                            </thead>
                                            <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                                                {permissions.into_iter().map(|perm| {
                                                    let perm_label = perm.label.clone();
                                                    let perm_key = perm.key.clone();
                                                    let perm_key_for_check = perm.key.clone();
                                                    let roles_for_row = roles_for_body.clone();
                                                    view! {
                                                        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/30">
                                                            <td class="px-4 py-2 text-sm text-gray-900 dark:text-white sticky left-0 bg-white dark:bg-gray-800">
                                                                <div class="font-medium">{perm_label}</div>
                                                                <div class="text-xs text-gray-500 dark:text-gray-400 font-mono">{perm_key}</div>
                                                            </td>
                                                            {roles_for_row.iter().map(|role| {
                                                                let has_perm = role.permissions.contains(&perm_key_for_check);
                                                                view! {
                                                                    <td class="px-3 py-2 text-center">
                                                                        <div class=format!(
                                                                            "w-6 h-6 mx-auto rounded-full flex items-center justify-center {}",
                                                                            if has_perm {
                                                                                "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400"
                                                                            } else {
                                                                                "bg-gray-100 dark:bg-gray-700 text-gray-400 dark:text-gray-500"
                                                                            }
                                                                        )>
                                                                            {if has_perm { "✓" } else { "—" }}
                                                                        </div>
                                                                    </td>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </tr>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            }.into_any()
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
    }
}

/// Модалка создания/редактирования роли.
#[component]
fn RoleModal(
    role: Option<Role>,
    on_close: impl Fn() + Send + Sync + 'static,
    on_saved: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_saved = StoredValue::new(on_saved);

    let is_editing = role.is_some();
    let role_id = role.as_ref().map(|r| r.id);
    let (name, set_name) = signal(role.as_ref().map(|r| r.name.clone()).unwrap_or_default());
    let (key, set_key) = signal(role.as_ref().map(|r| r.key.clone()).unwrap_or_default());
    let (description, set_description) = signal(role.as_ref().map(|r| r.description.clone()).unwrap_or_default());
    let (selected_permissions, set_selected_permissions) = signal(
        role.as_ref().map(|r| r.permissions.clone()).unwrap_or_default()
    );

    let all_permissions = get_all_permissions();

    let toggle_permission = move |perm_key: String| {
        let mut current = selected_permissions.get();
        if current.contains(&perm_key) {
            current.retain(|p| p != &perm_key);
        } else {
            current.push(perm_key);
        }
        set_selected_permissions.set(current);
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let _ = save_role(SaveRolePayload {
                id: role_id,
                name: name.get(),
                key: key.get(),
                description: description.get(),
                permissions: selected_permissions.get(),
            }).await;
            on_saved.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-3xl w-full max-h-[90vh] overflow-hidden flex flex-col">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">
                        {if is_editing { "Редактировать роль" } else { "Создать роль" }}
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

                <form on:submit=on_submit class="flex-1 overflow-y-auto p-6 space-y-4">
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Название *"</label>
                            <input
                                type="text"
                                prop:value=name
                                on:input=move |ev| set_name.set(event_target_value(&ev))
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                required
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Ключ (snake_case) *"</label>
                            <input
                                type="text"
                                prop:value=key
                                on:input=move |ev| set_key.set(event_target_value(&ev))
                                pattern="[a-z0-9_]+"
                                disabled=is_editing
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm disabled:opacity-50"
                                required
                            />
                        </div>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Описание"</label>
                        <textarea
                            prop:value=description
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                            rows="2"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        ></textarea>
                    </div>

                    <div>
                        <div class="flex items-center justify-between mb-3">
                            <label class="text-sm font-medium text-gray-700 dark:text-gray-300">"Разрешения"</label>
                            <span class="text-xs text-gray-500 dark:text-gray-400">
                                "Выбрано: " {selected_permissions.get().len()} " / " {all_permissions.len()}
                            </span>
                        </div>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-2 max-h-96 overflow-y-auto p-3 border border-gray-200 dark:border-gray-700 rounded-lg">
                            {all_permissions.into_iter().map(|perm| {
                                let perm_key_for_check = perm.key.clone();
                                let perm_key_for_toggle = perm.key.clone();
                                let perm_key_display = perm.key.clone();
                                let perm_label = perm.label.clone();
                                // ✅ Читаем значение из сигнала ЗДЕСЬ, чтобы передать в view!
                                let is_selected = selected_permissions.get().contains(&perm_key_for_check);
                                view! {
                                    <label class=format!(
                                        "flex items-start gap-2 p-2 rounded cursor-pointer transition-colors {}",
                                        if is_selected {
                                            "bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800"
                                        } else {
                                            "hover:bg-gray-50 dark:hover:bg-gray-700/50 border border-transparent"
                                        }
                                    )>
                                        <input
                                            type="checkbox"
                                            prop:checked=is_selected
                                            on:change=move |_| toggle_permission(perm_key_for_toggle.clone())
                                            class="mt-0.5"
                                        />
                                        <div class="flex-1 min-w-0">
                                            <div class="text-sm font-medium text-gray-900 dark:text-white">{perm_label}</div>
                                            <div class="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">{perm_key_display}</div>
                                        </div>
                                    </label>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button type="submit" class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors">
                            {if is_editing { "Сохранить" } else { "Создать роль" }}
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

/// Модалка деталей роли.
#[component]
fn RoleDetailModal(
    role: Role,
    on_close: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback (решает `use of moved value`)
    let on_close = StoredValue::new(on_close);

    // ✅ Клонируем поля до view!
    let role_key_for_color = role.key.clone();
    let role_key_for_icon = role.key.clone();
    let name = role.name.clone();
    let key = role.key.clone();
    let description = role.description.clone();
    let permissions = role.permissions.clone();
    let permissions_count = permissions.len();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-hidden flex flex-col">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <div class="flex items-center gap-3">
                        <div class=format!(
                            "w-12 h-12 rounded-xl flex items-center justify-center text-2xl {}",
                            get_role_color(&role_key_for_color)
                        )>
                            {get_role_icon(&role_key_for_icon)}
                        </div>
                        <div>
                            <h3 class="text-xl font-bold text-gray-900 dark:text-white">{name}</h3>
                            <p class="text-sm text-gray-500 dark:text-gray-400 font-mono">{key}</p>
                        </div>
                    </div>
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                <div class="flex-1 overflow-y-auto p-6 space-y-4">
                    <div>
                        <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Описание"</div>
                        <p class="text-sm text-gray-900 dark:text-white">{description}</p>
                    </div>

                    <div>
                        <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                            "Разрешения (" {permissions_count} ")"
                        </div>
                        <div class="flex flex-wrap gap-2">
                            {permissions.into_iter().map(|perm| {
                                view! {
                                    <span class="inline-flex items-center px-2 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 text-xs font-mono rounded">
                                        {perm}
                                    </span>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>

                <div class="p-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50 flex justify-end">
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                    >
                        "Закрыть"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Toast.
///
/// ⚠️ Авто-скрытие убрано (нет `gloo_timers` в зависимостях).
/// Пользователь закрывает вручную по кнопке «X».
#[component]
fn Toast(message: ToastMessage, on_close: impl Fn() + Send + Sync + 'static) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_close = StoredValue::new(on_close);

    let (bg_color, icon) = match message.variant {
        ToastVariant::Success => ("bg-green-600", "✓"),
        ToastVariant::Error => ("bg-red-600", "✗"),
        ToastVariant::Info => ("bg-blue-600", "ℹ"),
    };

    let msg = message.message.clone();

    view! {
        <div class="fixed bottom-4 right-4 z-50">
            <div class=format!("{} text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-3 min-w-[280px]", bg_color)>
                <span class="text-lg font-bold">{icon}</span>
                <span class="text-sm font-medium flex-1">{msg}</span>
                <button
                    on:click=move |_| on_close.with_value(|f| f())
                    class="text-white/80 hover:text-white"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                </button>
            </div>
        </div>
    }
}

// Скелетоны

#[component]
fn StatsSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6 animate-pulse">
            <div class="h-24 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-24 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-24 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

#[component]
fn TableSkeleton() -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden animate-pulse">
            <div class="h-12 bg-gray-200 dark:bg-gray-700"></div>
            <div class="divide-y divide-gray-200 dark:divide-gray-700">
                {(0..5).map(|_| view! { <div class="h-20 bg-gray-50 dark:bg-gray-800/50"></div> }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}

// Вспомогательные функции

fn get_role_icon(key: &str) -> &'static str {
    match key {
        "admin" => "👑",
        "instructor" => "👨‍🏫",
        "tutor" | "mentor" => "🧑‍🏫",
        "student" => "🎓",
        "observer" => "👁️",
        _ => "👤",
    }
}

fn get_role_color(key: &str) -> &'static str {
    match key {
        "admin" => "bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400",
        "instructor" => "bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400",
        "tutor" | "mentor" => "bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400",
        "student" => "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400",
        "observer" => "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400",
        _ => "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400",
    }
}

fn get_all_permissions() -> Vec<Permission> {
    vec![
        Permission { key: "courses.create".to_string(), label: "Создание курсов".to_string() },
        Permission { key: "courses.edit".to_string(), label: "Редактирование курсов".to_string() },
        Permission { key: "courses.delete".to_string(), label: "Удаление курсов".to_string() },
        Permission { key: "courses.publish".to_string(), label: "Публикация курсов".to_string() },
        Permission { key: "users.manage".to_string(), label: "Управление пользователями".to_string() },
        Permission { key: "users.view".to_string(), label: "Просмотр пользователей".to_string() },
        Permission { key: "batches.manage".to_string(), label: "Управление потоками".to_string() },
        Permission { key: "certificates.view".to_string(), label: "Просмотр сертификатов".to_string() },
        Permission { key: "analytics.view".to_string(), label: "Просмотр аналитики".to_string() },
        Permission { key: "settings.manage".to_string(), label: "Управление настройками".to_string() },
        Permission { key: "audit.view".to_string(), label: "Просмотр аудит-логов".to_string() },
        Permission { key: "billing.manage".to_string(), label: "Управление биллингом".to_string() },
    ]
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Роль пользователя.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Идентификатор.
    pub id: Uuid,
    /// Ключ (snake_case).
    pub key: String,
    /// Название.
    pub name: String,
    /// Описание.
    pub description: String,
    /// Системная (нельзя удалить).
    pub is_system: bool,
    /// Ключи разрешений.
    pub permissions: Vec<String>,
    /// Количество разрешений (денормализация для UI).
    pub permissions_count: usize,
    /// Количество пользователей с этой ролью.
    pub user_count: usize,
}

/// Разрешение (permission).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// Ключ.
    pub key: String,
    /// Человекочитаемое название.
    pub label: String,
}

/// Назначение роли пользователю.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleAssignment {
    /// ID пользователя.
    pub user_id: Uuid,
    /// Имя пользователя.
    pub user_name: String,
    /// Email пользователя.
    pub user_email: String,
    /// Ключ роли.
    pub role_key: String,
    /// Название роли.
    pub role_name: String,
    /// ID тенанта.
    pub tenant_id: Uuid,
    /// Название тенанта.
    pub tenant_name: String,
    /// Когда назначена.
    pub assigned_at: chrono::DateTime<chrono::Utc>,
}

/// Payload для сохранения роли.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveRolePayload {
    /// ID (None для создания).
    pub id: Option<Uuid>,
    /// Название.
    pub name: String,
    /// Ключ.
    pub key: String,
    /// Описание.
    pub description: String,
    /// Разрешения.
    pub permissions: Vec<String>,
}

/// Вариант toast-сообщения.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastVariant {
    /// Успех.
    Success,
    /// Ошибка.
    Error,
    /// Информация.
    Info,
}

/// Toast-сообщение.
#[derive(Clone, Debug)]
pub struct ToastMessage {
    /// Текст.
    pub message: String,
    /// Вариант.
    pub variant: ToastVariant,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

#[server(GetAllRoles, "/api")]
pub async fn fetch_all_roles() -> Result<Vec<Role>, ServerFnError> {
    // TODO: SELECT из roles + агрегация user_count и permissions_count
    // На этапе вёрстки возвращаем 5 предопределённых ролей.

    let mock = vec![
        Role {
            id: Uuid::new_v4(),
            key: "admin".to_string(),
            name: "Администратор".to_string(),
            description: "Полный доступ ко всем функциям платформы".to_string(),
            is_system: true,
            permissions: vec![
                "courses.create".to_string(),
                "courses.edit".to_string(),
                "courses.delete".to_string(),
                "courses.publish".to_string(),
                "users.manage".to_string(),
                "users.view".to_string(),
                "batches.manage".to_string(),
                "certificates.view".to_string(),
                "analytics.view".to_string(),
                "settings.manage".to_string(),
                "audit.view".to_string(),
                "billing.manage".to_string(),
            ],
            permissions_count: 12,
            user_count: 3,
        },
        Role {
            id: Uuid::new_v4(),
            key: "instructor".to_string(),
            name: "Инструктор".to_string(),
            description: "Управление курсами и потоками, просмотр аналитики".to_string(),
            is_system: true,
            permissions: vec![
                "courses.create".to_string(),
                "courses.edit".to_string(),
                "courses.publish".to_string(),
                "users.view".to_string(),
                "batches.manage".to_string(),
                "analytics.view".to_string(),
            ],
            permissions_count: 6,
            user_count: 12,
        },
        Role {
            id: Uuid::new_v4(),
            key: "tutor".to_string(),
            name: "Тьютор".to_string(),
            description: "Проверка заданий, ответы на вопросы студентов".to_string(),
            is_system: true,
            permissions: vec![
                "users.view".to_string(),
                "certificates.view".to_string(),
                "analytics.view".to_string(),
            ],
            permissions_count: 3,
            user_count: 8,
        },
        Role {
            id: Uuid::new_v4(),
            key: "student".to_string(),
            name: "Студент".to_string(),
            description: "Базовый доступ: обучение и просмотр своих результатов".to_string(),
            is_system: true,
            permissions: vec![],
            permissions_count: 0,
            user_count: 247,
        },
        Role {
            id: Uuid::new_v4(),
            key: "observer".to_string(),
            name: "Наблюдатель".to_string(),
            description: "Просмотр отчётов без вмешательства в обучение".to_string(),
            is_system: true,
            permissions: vec![
                "users.view".to_string(),
                "analytics.view".to_string(),
            ],
            permissions_count: 2,
            user_count: 5,
        },
    ];

    Ok(mock)
}

#[server(GetRoleAssignments, "/api")]
pub async fn fetch_role_assignments() -> Result<Vec<UserRoleAssignment>, ServerFnError> {
    // TODO: SELECT из user_roles + JOIN users + tenants
    // На этапе вёрстки возвращаем 4 мок-назначения.

    let now = chrono::Utc::now();
    let mock = vec![
        UserRoleAssignment {
            user_id: Uuid::new_v4(),
            user_name: "Иван Студентов".to_string(),
            user_email: "student@example.com".to_string(),
            role_key: "student".to_string(),
            role_name: "Студент".to_string(),
            tenant_id: Uuid::new_v4(),
            tenant_name: "Demo Organization".to_string(),
            assigned_at: now - chrono::Duration::days(30),
        },
        UserRoleAssignment {
            user_id: Uuid::new_v4(),
            user_name: "Мария Преподавателева".to_string(),
            user_email: "instructor@example.com".to_string(),
            role_key: "instructor".to_string(),
            role_name: "Инструктор".to_string(),
            tenant_id: Uuid::new_v4(),
            tenant_name: "Demo Organization".to_string(),
            assigned_at: now - chrono::Duration::days(60),
        },
        UserRoleAssignment {
            user_id: Uuid::new_v4(),
            user_name: "Пётр Тьюторов".to_string(),
            user_email: "tutor@example.com".to_string(),
            role_key: "tutor".to_string(),
            role_name: "Тьютор".to_string(),
            tenant_id: Uuid::new_v4(),
            tenant_name: "Demo Organization".to_string(),
            assigned_at: now - chrono::Duration::days(45),
        },
        UserRoleAssignment {
            user_id: Uuid::new_v4(),
            user_name: "Ольга Админова".to_string(),
            user_email: "admin@example.com".to_string(),
            role_key: "admin".to_string(),
            role_name: "Администратор".to_string(),
            tenant_id: Uuid::new_v4(),
            tenant_name: "Demo Organization".to_string(),
            assigned_at: now - chrono::Duration::days(90),
        },
    ];

    Ok(mock)
}

#[server(SaveRole, "/api")]
pub async fn save_role(payload: SaveRolePayload) -> Result<Role, ServerFnError> {
    // TODO: INSERT/UPDATE roles + role_permissions
    // На этапе вёрстки возвращаем мок-роль из payload.

    let role = Role {
        id: payload.id.unwrap_or_else(Uuid::new_v4),
        key: payload.key,
        name: payload.name,
        description: payload.description,
        is_system: false,
        permissions: payload.permissions.clone(),
        permissions_count: payload.permissions.len(),
        user_count: 0,
    };

    Ok(role)
}