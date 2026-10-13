// crates/client/src/admin/pages/licenses.rs
//! Управление лицензиями (Этап 29 Air-gapped Core).
//!
//! UI для управления лицензиями коробочных поставок: просмотр активной лицензии,
//! генерация новых лицензий, валидация, управление привязками (machine-id, domain, user),
//! мониторинг потребления лимитов, история активаций.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ `License`, `LicenseBinding`, `LicenseStatus`, `LicenseType` — локальные DTO:
//! они ещё не вынесены в `rust_lms_shared::models`. При интеграции с Этапом 29
//! — перенести в shared (по аналогии с Course/Node/Certificate).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Страница управления лицензиями.
#[component]
pub fn AdminLicenseList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let current_license_resource = Resource::new(|| (), |_| fetch_current_license());
    let licenses_resource = Resource::new(|| (), |_| fetch_all_licenses());
    // activations_resource пока не используется в UI, но оставлен для будущего
    let _activations_resource = Resource::new(|| (), |_| fetch_license_activations());

    // Сигналы состояния
    let (show_generate_modal, set_show_generate_modal) = signal(false);
    let (show_validate_modal, set_show_validate_modal) = signal(false);
    let (selected_license, set_selected_license) = signal(None::<License>);
    let (toast, set_toast) = signal(None::<ToastMessage>);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
                        <span class="text-2xl">"🔑"</span>
                        // i18n: t!("admin.licenses.title")
                        "Лицензии"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.licenses.subtitle")
                        "Управление лицензиями для коробочных поставок"
                    </p>
                </div>

                <div class="flex items-center gap-2">
                    <button
                        on:click=move |_| set_show_validate_modal.set(true)
                        class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors flex items-center gap-2"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"></path>
                        </svg>
                        "Проверить лицензию"
                    </button>
                    <button
                        on:click=move |_| set_show_generate_modal.set(true)
                        class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                        </svg>
                        "Сгенерировать"
                    </button>
                </div>
            </div>

            // Текущая активная лицензия
            <Suspense fallback=CurrentLicenseSkeleton>
                {move || {
                    current_license_resource.get().map(|result| {
                        match result {
                            Ok(license) => view! {
                                <CurrentLicenseCard license=license />
                            }.into_any(),
                            Err(_) => view! {
                                <div class="mb-6 p-6 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-xl">
                                    <div class="flex items-start gap-3">
                                        <div class="text-3xl">"⚠️"</div>
                                        <div>
                                            <h3 class="text-lg font-semibold text-yellow-900 dark:text-yellow-200 mb-1">
                                                "Лицензия не активна"
                                            </h3>
                                            <p class="text-sm text-yellow-800 dark:text-yellow-300 mb-3">
                                                "Платформа работает в демо-режиме с ограничениями. Активируйте лицензию для полного функционала."
                                            </p>
                                            <button
                                                on:click=move |_| set_show_validate_modal.set(true)
                                                class="px-4 py-2 bg-yellow-600 text-white font-medium rounded-lg hover:bg-yellow-700 transition-colors"
                                            >
                                                "Активировать лицензию"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Статистика
            <Suspense fallback=StatsSkeleton>
                {move || {
                    licenses_resource.get().map(|result| {
                        match result {
                            Ok(licenses) => {
                                let total = licenses.len();
                                let active = licenses.iter().filter(|l| l.status == LicenseStatus::Active).count();
                                let expired = licenses.iter().filter(|l| l.status == LicenseStatus::Expired).count();

                                view! {
                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Всего лицензий"</div>
                                            <div class="text-2xl font-bold text-gray-900 dark:text-white">{total}</div>
                                        </div>
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Активных"</div>
                                            <div class="text-2xl font-bold text-green-600 dark:text-green-400">{active}</div>
                                        </div>
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Истекло"</div>
                                            <div class="text-2xl font-bold text-red-600 dark:text-red-400">{expired}</div>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            Err(_) => view! { <div></div> }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Список всех лицензий
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h2 class="text-lg font-semibold text-gray-900 dark:text-white">"История лицензий"</h2>
                </div>
                <Suspense fallback=TableSkeleton>
                    {move || {
                        licenses_resource.get().map(|result| {
                            match result {
                                Ok(licenses) => {
                                    if licenses.is_empty() {
                                        view! {
                                            <div class="text-center py-16">
                                                <div class="text-5xl mb-4">"🔑"</div>
                                                <p class="text-gray-500 dark:text-gray-400 text-lg mb-1">"Лицензии не найдены"</p>
                                                <p class="text-gray-400 dark:text-gray-500 text-sm">"Сгенерируйте первую лицензию"</p>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="overflow-x-auto">
                                                <table class="w-full">
                                                    <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                                                        <tr>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"ID"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Тип"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Статус"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Привязка"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Срок действия"</th>
                                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Действия"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                                                        {licenses.into_iter().map(|license| {
                                                            let license_for_view = license.clone();
                                                            view! {
                                                                <LicenseRow
                                                                    license=license
                                                                    on_view=move || set_selected_license.set(Some(license_for_view.clone()))
                                                                />
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </tbody>
                                                </table>
                                            </div>
                                        }.into_any()
                                    }
                                }
                                Err(e) => view! {
                                    <div class="p-6 text-red-600 dark:text-red-400">{e.to_string()}</div>
                                }.into_any(),
                            }
                        })
                    }}
                </Suspense>
            </div>

            // Модалка генерации лицензии
            <Show when=move || show_generate_modal.get()>
                <GenerateLicenseModal
                    on_close=move || set_show_generate_modal.set(false)
                    on_generated=move || {
                        set_show_generate_modal.set(false);
                        licenses_resource.refetch();
                        current_license_resource.refetch();
                        set_toast.set(Some(ToastMessage {
                            message: "Лицензия сгенерирована".to_string(),
                            variant: ToastVariant::Success,
                        }));
                    }
                />
            </Show>

            // Модалка валидации
            <Show when=move || show_validate_modal.get()>
                <ValidateLicenseModal
                    on_close=move || set_show_validate_modal.set(false)
                    on_validated=move || {
                        set_show_validate_modal.set(false);
                        current_license_resource.refetch();
                        set_toast.set(Some(ToastMessage {
                            message: "Лицензия активирована".to_string(),
                            variant: ToastVariant::Success,
                        }));
                    }
                />
            </Show>

            // Модалка деталей лицензии
            <Show when=move || selected_license.get().is_some()>
                <LicenseDetailModal
                    license=selected_license.get().unwrap()
                    on_close=move || set_selected_license.set(None)
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

/// Карточка текущей активной лицензии.
#[component]
fn CurrentLicenseCard(license: License) -> impl IntoView {
    let days_left = license.days_until_expiration;
    let urgency_color = if days_left <= 7 {
        "border-red-300 dark:border-red-600 bg-red-50/30 dark:bg-red-900/10"
    } else if days_left <= 30 {
        "border-yellow-300 dark:border-yellow-600 bg-yellow-50/30 dark:bg-yellow-900/10"
    } else {
        "border-green-300 dark:border-green-600 bg-green-50/30 dark:bg-green-900/10"
    };

    let usage_pct = (license.users_used as f64 / license.users_limit.max(1) as f64 * 100.0).min(100.0);
    let license_id_short: String = license.id.to_string().chars().take(8).collect();
    let expires_str = license.expires_at.format("%d.%m.%Y").to_string();
    let type_label = get_license_type_label(&license.license_type);
    let binding_label = get_binding_label(&license.binding);
    let users_used = license.users_used;
    let users_limit = license.users_limit;

    // Цвет текста для срока действия
    let days_color = if days_left <= 7 {
        "text-red-600 dark:text-red-400"
    } else if days_left <= 30 {
        "text-yellow-600 dark:text-yellow-400"
    } else {
        "text-green-600 dark:text-green-400"
    };
    let days_text = if days_left > 0 {
        format!("{} дн. осталось", days_left)
    } else {
        "Истекла".to_string()
    };

    // Цвет прогресс-бара
    let bar_color = if usage_pct > 90.0 {
        "bg-red-500 h-1.5 rounded-full"
    } else if usage_pct > 70.0 {
        "bg-yellow-500 h-1.5 rounded-full"
    } else {
        "bg-green-500 h-1.5 rounded-full"
    };

    view! {
        <div class=format!("mb-6 p-6 border-2 rounded-xl {}", urgency_color)>
            <div class="flex items-start justify-between gap-4 mb-4">
                <div class="flex items-start gap-4">
                    <div class="w-14 h-14 rounded-xl bg-gradient-to-br from-green-500 to-emerald-600 flex items-center justify-center text-3xl">
                        "🔑"
                    </div>
                    <div>
                        <h2 class="text-xl font-bold text-gray-900 dark:text-white mb-1">
                            "Активная лицензия"
                        </h2>
                        <p class="text-sm text-gray-600 dark:text-gray-400 font-mono">
                            "ID: " {license_id_short} "..."
                        </p>
                    </div>
                </div>
                <div class="text-right">
                    <div class=format!("text-sm font-semibold {}", days_color)>
                        {days_text}
                    </div>
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        "До " {expires_str}
                    </div>
                </div>
            </div>

            // Лимиты
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div class="p-3 bg-white/50 dark:bg-gray-800/50 rounded-lg">
                    <div class="text-xs text-gray-600 dark:text-gray-400 mb-1">"Пользователи"</div>
                    <div class="text-lg font-bold text-gray-900 dark:text-white mb-2">
                        {users_used} " / " {users_limit}
                    </div>
                    <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                        <div
                            class=bar_color
                            style:width=format!("{}%", usage_pct)
                        ></div>
                    </div>
                </div>

                <div class="p-3 bg-white/50 dark:bg-gray-800/50 rounded-lg">
                    <div class="text-xs text-gray-600 dark:text-gray-400 mb-1">"Тип лицензии"</div>
                    <div class="text-lg font-bold text-gray-900 dark:text-white">
                        {type_label}
                    </div>
                </div>

                <div class="p-3 bg-white/50 dark:bg-gray-800/50 rounded-lg">
                    <div class="text-xs text-gray-600 dark:text-gray-400 mb-1">"Привязка"</div>
                    <div class="text-sm font-mono text-gray-900 dark:text-white truncate">
                        {binding_label}
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Строка таблицы лицензии.
#[component]
fn LicenseRow(license: License, on_view: impl Fn() + Send + Sync + 'static) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_view = StoredValue::new(on_view);

    let status_badge = match license.status {
        LicenseStatus::Active => ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "✅ Активна"),
        LicenseStatus::Expired => ("bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300", "⏰ Истекла"),
        LicenseStatus::Revoked => ("bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200", "🚫 Отозвана"),
        LicenseStatus::Trial => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "🎁 Trial"),
    };

    let id_short: String = license.id.to_string().chars().take(8).collect();
    let type_label = get_license_type_label(&license.license_type);
    let binding_label = get_binding_label(&license.binding);
    let expires_str = license.expires_at.format("%d.%m.%Y").to_string();

    view! {
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors">
            <td class="px-4 py-3">
                <code class="text-xs font-mono text-gray-700 dark:text-gray-300">
                    {id_short} "..."
                </code>
            </td>
            <td class="px-4 py-3">
                <span class="text-sm text-gray-900 dark:text-white">
                    {type_label}
                </span>
            </td>
            <td class="px-4 py-3">
                <span class=format!("inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {}", status_badge.0)>
                    {status_badge.1}
                </span>
            </td>
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                {binding_label}
            </td>
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                {expires_str}
            </td>
            <td class="px-4 py-3 text-right">
                <button
                    on:click=move |_| on_view.with_value(|f| f())
                    class="text-sm text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 font-medium"
                >
                    "Подробнее"
                </button>
            </td>
        </tr>
    }
}

/// Модалка генерации новой лицензии.
#[component]
fn GenerateLicenseModal(
    on_close: impl Fn() + Send + Sync + 'static,
    on_generated: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_generated = StoredValue::new(on_generated);

    let (license_type, set_license_type) = signal(LicenseType::Standard);
    let (binding_type, set_binding_type) = signal(BindingType::MachineId);
    let (binding_value, set_binding_value) = signal(String::new());
    let (duration_days, set_duration_days) = signal(365u32);
    let (users_limit, set_users_limit) = signal(100u32);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let _ = generate_license(GenerateLicensePayload {
                license_type: license_type.get(),
                binding: LicenseBinding {
                    binding_type: binding_type.get(),
                    value: binding_value.get(),
                },
                duration_days: duration_days.get(),
                users_limit: users_limit.get(),
            }).await;
            on_generated.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">"Сгенерировать лицензию"</h3>
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
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">"Тип лицензии"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <TypeCard
                                icon="🎁"
                                label="Trial"
                                active=move || license_type.get() == LicenseType::Trial
                                on_click=move || set_license_type.set(LicenseType::Trial)
                            />
                            <TypeCard
                                icon="📦"
                                label="Standard"
                                active=move || license_type.get() == LicenseType::Standard
                                on_click=move || set_license_type.set(LicenseType::Standard)
                            />
                            <TypeCard
                                icon="🏢"
                                label="Enterprise"
                                active=move || license_type.get() == LicenseType::Enterprise
                                on_click=move || set_license_type.set(LicenseType::Enterprise)
                            />
                        </div>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">"Тип привязки"</label>
                        <select
                            on:change=move |ev| {
                                set_binding_type.set(match event_target_value(&ev).as_str() {
                                    "domain" => BindingType::Domain,
                                    "user" => BindingType::User,
                                    _ => BindingType::MachineId,
                                });
                            }
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        >
                            <option value="machine_id">"🖥️ Machine ID"</option>
                            <option value="domain">"🌐 Домен"</option>
                            <option value="user">"👤 Пользователь"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Значение привязки *"</label>
                        <input
                            type="text"
                            prop:value=binding_value
                            on:input=move |ev| set_binding_value.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                            placeholder="a1b2c3d4-e5f6-..."
                            required
                        />
                    </div>

                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Срок действия (дни)"</label>
                            <input
                                type="number"
                                min="1"
                                prop:value=duration_days
                                on:input=move |ev| {
                                    if let Ok(val) = event_target_value(&ev).parse() {
                                        set_duration_days.set(val);
                                    }
                                }
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Лимит пользователей"</label>
                            <input
                                type="number"
                                min="1"
                                prop:value=users_limit
                                on:input=move |ev| {
                                    if let Ok(val) = event_target_value(&ev).parse() {
                                        set_users_limit.set(val);
                                    }
                                }
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            />
                        </div>
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                        >
                            "Сгенерировать"
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

/// Карточка типа лицензии.
#[component]
fn TypeCard(
    icon: &'static str,
    label: &'static str,
    active: impl Fn() -> bool + Send + Sync + 'static,
    on_click: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let active = StoredValue::new(active);
    let on_click = StoredValue::new(on_click);

    view! {
        <button
            type="button"
            on:click=move |_| on_click.with_value(|f| f())
            class=move || {
                format!(
                    "p-3 border-2 rounded-lg text-center transition-all {}",
                    if active.with_value(|f| f()) {
                        "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                    } else {
                        "border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600"
                    }
                )
            }
        >
            <div class="text-2xl mb-1">{icon}</div>
            <div class="text-sm font-medium text-gray-900 dark:text-white">{label}</div>
        </button>
    }
}

/// Модалка валидации лицензии.
#[component]
fn ValidateLicenseModal(
    on_close: impl Fn() + Send + Sync + 'static,
    on_validated: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_validated = StoredValue::new(on_validated);

    let (license_key, set_license_key) = signal(String::new());
    let (validation_result, set_validation_result) = signal(None::<ValidationResult>);

    let handle_validate = move || {
        spawn_local(async move {
            let _ = validate_license(license_key.get()).await;
            set_validation_result.set(Some(ValidationResult { valid: true }));
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-md w-full">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">"Активировать лицензию"</h3>
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                <div class="p-6 space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Лицензионный ключ"</label>
                        <textarea
                            prop:value=license_key
                            on:input=move |ev| set_license_key.set(event_target_value(&ev))
                            rows="4"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                            placeholder="Вставьте лицензионный ключ..."
                        ></textarea>
                    </div>

                    <Show when=move || validation_result.get().is_some()>
                        <div class="p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
                            <div class="flex items-start gap-2">
                                <div class="text-xl">"✅"</div>
                                <div>
                                    <p class="text-sm font-semibold text-green-800 dark:text-green-300">"Лицензия валидна"</p>
                                    <p class="text-xs text-green-700 dark:text-green-400 mt-1">"Готово к активации"</p>
                                </div>
                            </div>
                        </div>
                    </Show>

                    <div class="flex gap-3">
                        <button
                            on:click=move |_| handle_validate()
                            class="flex-1 px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                        >
                            "Проверить"
                        </button>
                        <button
                            on:click=move |_| on_validated.with_value(|f| f())
                            disabled=move || validation_result.get().is_none()
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        >
                            "Активировать"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Модалка деталей лицензии.
#[component]
fn LicenseDetailModal(
    license: License,
    on_close: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_close = StoredValue::new(on_close);

    let id_str = license.id.to_string();
    let type_str = get_license_type_label(&license.license_type).to_string();
    let status_str = format!("{:?}", license.status);
    let created_str = license.created_at.format("%d.%m.%Y %H:%M").to_string();
    let expires_str = license.expires_at.format("%d.%m.%Y %H:%M").to_string();
    let users_limit = license.users_limit.to_string();
    let users_used = license.users_used.to_string();
    let binding_str = get_binding_label(&license.binding);
    let public_key = license.public_key.clone();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-hidden flex flex-col">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">"Детали лицензии"</h3>
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
                    <div class="grid grid-cols-2 gap-4">
                        <DetailRow label="ID" value=id_str />
                        <DetailRow label="Тип" value=type_str />
                        <DetailRow label="Статус" value=status_str />
                        <DetailRow label="Создана" value=created_str />
                        <DetailRow label="Истекает" value=expires_str />
                        <DetailRow label="Лимит пользователей" value=users_limit />
                        <DetailRow label="Использовано" value=users_used />
                        <DetailRow label="Привязка" value=binding_str />
                    </div>

                    <div>
                        <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">"Публичный ключ"</div>
                        <div class="p-3 bg-gray-900 text-gray-100 rounded-lg overflow-x-auto">
                            <pre class="text-xs font-mono whitespace-pre-wrap break-all">{public_key}</pre>
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

/// Строка деталей.
#[component]
fn DetailRow(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div>
            <div class="text-xs text-gray-600 dark:text-gray-400 mb-1">{label}</div>
            <div class="text-sm font-medium text-gray-900 dark:text-white break-all">{value}</div>
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
fn CurrentLicenseSkeleton() -> impl IntoView {
    view! {
        <div class="mb-6 h-48 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
    }
}

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
        <div class="divide-y divide-gray-200 dark:divide-gray-700 animate-pulse">
            {(0..5).map(|_| view! { <div class="h-16 bg-gray-50 dark:bg-gray-800/50"></div> }).collect::<Vec<_>>()}
        </div>
    }
}

// Вспомогательные функции

fn get_license_type_label(t: &LicenseType) -> &'static str {
    match t {
        LicenseType::Trial => "🎁 Trial",
        LicenseType::Standard => "📦 Standard",
        LicenseType::Enterprise => "🏢 Enterprise",
    }
}

fn get_binding_label(b: &LicenseBinding) -> String {
    match b.binding_type {
        BindingType::MachineId => format!("🖥️ {}", b.value),
        BindingType::Domain => format!("🌐 {}", b.value),
        BindingType::User => format!("👤 {}", b.value),
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Тип лицензии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseType {
    /// Пробная.
    Trial,
    /// Стандартная.
    Standard,
    /// Корпоративная.
    Enterprise,
}

/// Статус лицензии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseStatus {
    /// Активна.
    Active,
    /// Истекла.
    Expired,
    /// Отозвана.
    Revoked,
    /// Trial-период.
    Trial,
}

/// Тип привязки.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingType {
    /// Machine ID.
    MachineId,
    /// Домен.
    Domain,
    /// Пользователь.
    User,
}

/// Привязка лицензии.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseBinding {
    /// Тип привязки.
    pub binding_type: BindingType,
    /// Значение привязки.
    pub value: String,
}

/// Лицензия.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// Идентификатор.
    pub id: Uuid,
    /// Тип.
    pub license_type: LicenseType,
    /// Статус.
    pub status: LicenseStatus,
    /// Привязка.
    pub binding: LicenseBinding,
    /// Создана.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Истекает.
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Дней до истечения.
    pub days_until_expiration: i64,
    /// Лимит пользователей.
    pub users_limit: u32,
    /// Использовано пользователей.
    pub users_used: u32,
    /// Публичный ключ.
    pub public_key: String,
}

/// Payload для генерации лицензии.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateLicensePayload {
    /// Тип лицензии.
    pub license_type: LicenseType,
    /// Привязка.
    pub binding: LicenseBinding,
    /// Срок действия (дни).
    pub duration_days: u32,
    /// Лимит пользователей.
    pub users_limit: u32,
}

/// Результат валидации.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Валидна ли.
    pub valid: bool,
}

/// Активация лицензии.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseActivation {
    /// Когда активирована.
    pub activated_at: chrono::DateTime<chrono::Utc>,
    /// Machine ID.
    pub machine_id: String,
    /// IP-адрес.
    pub ip_address: String,
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

#[server(GetCurrentLicense, "/api")]
pub async fn fetch_current_license() -> Result<License, ServerFnError> {
    // TODO: Получение активной лицензии из БД/файла
    // На этапе вёрстки возвращаем мок-лицензию.
    let now = chrono::Utc::now();

    Ok(License {
        id: Uuid::new_v4(),
        license_type: LicenseType::Enterprise,
        status: LicenseStatus::Active,
        binding: LicenseBinding {
            binding_type: BindingType::Domain,
            value: "lms.example.com".to_string(),
        },
        created_at: now - chrono::Duration::days(180),
        expires_at: now + chrono::Duration::days(185),
        days_until_expiration: 185,
        users_limit: 500,
        users_used: 247,
        public_key: "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAGb9ECWmEzf6FQbrBZ9w7lshQhqowtrbLDFw4rXAxZuE=\n-----END PUBLIC KEY-----".to_string(),
    })
}

#[server(GetAllLicenses, "/api")]
pub async fn fetch_all_licenses() -> Result<Vec<License>, ServerFnError> {
    // TODO: SELECT из licenses ORDER BY created_at DESC
    // На этапе вёрстки возвращаем мок-данные.
    let now = chrono::Utc::now();

    let mock = vec![
        License {
            id: Uuid::new_v4(),
            license_type: LicenseType::Enterprise,
            status: LicenseStatus::Active,
            binding: LicenseBinding {
                binding_type: BindingType::Domain,
                value: "lms.example.com".to_string(),
            },
            created_at: now - chrono::Duration::days(180),
            expires_at: now + chrono::Duration::days(185),
            days_until_expiration: 185,
            users_limit: 500,
            users_used: 247,
            public_key: "-----BEGIN PUBLIC KEY-----\nEnterprise-key-1\n-----END PUBLIC KEY-----".to_string(),
        },
        License {
            id: Uuid::new_v4(),
            license_type: LicenseType::Standard,
            status: LicenseStatus::Expired,
            binding: LicenseBinding {
                binding_type: BindingType::MachineId,
                value: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
            },
            created_at: now - chrono::Duration::days(400),
            expires_at: now - chrono::Duration::days(35),
            days_until_expiration: -35,
            users_limit: 100,
            users_used: 87,
            public_key: "-----BEGIN PUBLIC KEY-----\nStandard-key-1\n-----END PUBLIC KEY-----".to_string(),
        },
        License {
            id: Uuid::new_v4(),
            license_type: LicenseType::Trial,
            status: LicenseStatus::Trial,
            binding: LicenseBinding {
                binding_type: BindingType::User,
                value: "admin@example.com".to_string(),
            },
            created_at: now - chrono::Duration::days(5),
            expires_at: now + chrono::Duration::days(25),
            days_until_expiration: 25,
            users_limit: 10,
            users_used: 3,
            public_key: "-----BEGIN PUBLIC KEY-----\nTrial-key-1\n-----END PUBLIC KEY-----".to_string(),
        },
    ];

    Ok(mock)
}

#[server(GetLicenseActivations, "/api")]
pub async fn fetch_license_activations() -> Result<Vec<LicenseActivation>, ServerFnError> {
    // TODO: SELECT из license_activations
    // На этапе вёрстки возвращаем пустой список.
    Ok(vec![])
}

#[server(GenerateLicense, "/api")]
pub async fn generate_license(payload: GenerateLicensePayload) -> Result<License, ServerFnError> {
    // TODO: Генерация Ed25519 подписанного токена
    // На этапе вёрстки возвращаем мок-лицензию из payload.
    let now = chrono::Utc::now();

    Ok(License {
        id: Uuid::new_v4(),
        license_type: payload.license_type,
        status: LicenseStatus::Active,
        binding: payload.binding,
        created_at: now,
        expires_at: now + chrono::Duration::days(payload.duration_days as i64),
        days_until_expiration: payload.duration_days as i64,
        users_limit: payload.users_limit,
        users_used: 0,
        public_key: "-----BEGIN PUBLIC KEY-----\nGenerated-key\n-----END PUBLIC KEY-----".to_string(),
    })
}

#[server(ValidateLicense, "/api")]
pub async fn validate_license(_key: String) -> Result<ValidationResult, ServerFnError> {
    // TODO: Валидация Ed25519-подписи лицензионного ключа
    // На этапе вёрстки всегда валидно.
    Ok(ValidationResult { valid: true })
}