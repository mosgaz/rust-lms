// crates/client/src/admin/pages/features.rs
//! Управление функциональными флагами (Feature Flags).
//!
//! Интеграция с Этапом 29 (Air-gapped Core). UI для управления глобальными
//! и тенант-специфичными флагами: включение/выключение функций, переопределения
//! для отдельных тенантов, каскадная логика (global → tenant override).
//! Поддержка горячего обновления без перезапуска сервера.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ `FeatureFlag` и `FeatureFlagScope` — локальные DTO: они ещё не вынесены
//! в `rust_lms_shared::models`. При интеграции с Этапом 29 — перенести в shared
//! (по аналогии с Course/Node/Certificate).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Страница управления функциональными флагами.
#[component]
pub fn AdminFeatureList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let flags_resource = Resource::new(|| (), |_| fetch_all_feature_flags());
    // overrides_resource пока не используется в UI, но оставлен для будущего
    let _overrides_resource = Resource::new(|| (), |_| fetch_tenant_overrides());

    // Сигналы состояния
    let (search_query, set_search_query) = signal(String::new());
    let (scope_filter, set_scope_filter) = signal(None::<FeatureFlagScope>);
    let (show_create_modal, set_show_create_modal) = signal(false);
    let (editing_flag, set_editing_flag) = signal(None::<FeatureFlag>);
    let (toast, set_toast) = signal(None::<ToastMessage>);
    let (_active_tab, _set_active_tab) = signal(FeaturesTab::Global);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
                        <span class="text-2xl">"🚩"</span>
                        // i18n: t!("admin.features.title")
                        "Функциональные флаги"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.features.subtitle")
                        "Управление доступностью функций для тенантов"
                    </p>
                </div>

                <button
                    on:click=move |_| set_show_create_modal.set(true)
                    class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    // i18n: t!("admin.features.create_button")
                    "Создать флаг"
                </button>
            </div>

            // Информационный баннер
            <div class="mb-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                <div class="flex items-start gap-3">
                    <svg class="w-5 h-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    <div class="text-sm text-blue-800 dark:text-blue-300">
                        <p class="font-medium mb-1">"Как работают функциональные флаги"</p>
                        <p>
                            "Флаги применяются каскадно: сначала проверяется переопределение для тенанта, "
                            "затем глобальное значение. Изменения применяются мгновенно без перезапуска сервера "
                            "через PostgreSQL LISTEN/NOTIFY."
                        </p>
                    </div>
                </div>
            </div>

            // Статистика
            <Suspense fallback=StatsSkeleton>
                {move || {
                    flags_resource.get().map(|result| {
                        match result {
                            Ok(flags) => {
                                let total = flags.len();
                                let enabled = flags.iter().filter(|f| f.is_enabled).count();
                                let with_overrides = flags.iter().filter(|f| f.has_overrides).count();

                                view! {
                                    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Всего флагов"</div>
                                            <div class="text-2xl font-bold text-gray-900 dark:text-white">{total}</div>
                                        </div>
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"Активных"</div>
                                            <div class="text-2xl font-bold text-green-600 dark:text-green-400">{enabled} " / " {total}</div>
                                        </div>
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                                            <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-1">"С переопределениями"</div>
                                            <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{with_overrides}</div>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            Err(_) => view! { <div></div> }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Панель фильтров
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4 mb-6">
                <div class="flex flex-col lg:flex-row items-stretch lg:items-center gap-3">
                    <div class="relative flex-1">
                        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                        </svg>
                        <input
                            type="text"
                            placeholder="Поиск по ключу или описанию..."
                            prop:value=search_query
                            on:input=move |ev| set_search_query.set(event_target_value(&ev))
                            class="w-full pl-10 pr-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm focus:ring-2 focus:ring-blue-500"
                        />
                    </div>

                    <select
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            set_scope_filter.set(match val.as_str() {
                                "global" => Some(FeatureFlagScope::Global),
                                "tenant" => Some(FeatureFlagScope::Tenant),
                                "kill_switch" => Some(FeatureFlagScope::KillSwitch),
                                _ => None,
                            });
                        }
                        class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                    >
                        <option value="">"Все области"</option>
                        <option value="global">"🌍 Глобальные"</option>
                        <option value="tenant">"🏢 Тенант-специфичные"</option>
                        <option value="kill_switch">"🚨 Kill switches"</option>
                    </select>
                </div>
            </div>

            // Основной контент
            <Suspense fallback=TableSkeleton>
                {move || {
                    flags_resource.get().map(|result| {
                        match result {
                            Ok(flags) => {
                                let filtered = flags.into_iter()
                                    .filter(|f| {
                                        let matches_search = search_query.get().is_empty() ||
                                            f.key.to_lowercase().contains(&search_query.get().to_lowercase()) ||
                                            f.description.to_lowercase().contains(&search_query.get().to_lowercase());
                                        let matches_scope = scope_filter.get().is_none() ||
                                            f.scope == scope_filter.get().unwrap();
                                        matches_search && matches_scope
                                    })
                                    .collect::<Vec<_>>();

                                if filtered.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <div class="text-5xl mb-4">"🚩"</div>
                                            <p class="text-gray-500 dark:text-gray-400 text-lg mb-1">"Флаги не найдены"</p>
                                            <p class="text-gray-400 dark:text-gray-500 text-sm">"Создайте первый функциональный флаг"</p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                                            <div class="overflow-x-auto">
                                                <table class="w-full">
                                                    <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                                                        <tr>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Флаг"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Область"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Статус"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Переопределения"</th>
                                                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Обновлён"</th>
                                                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Действия"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                                                        {filtered.into_iter().map(|flag| {
															// ✅ Клонируем flag до замыканий, чтобы избежать E0382
															let flag_for_edit = flag.clone();

															view! {
																<FeatureFlagRow
																	flag=flag
																	on_edit=move || set_editing_flag.set(Some(flag_for_edit.clone()))
																	on_toggle=move |flag_id, enabled| {
																		spawn_local(async move {
																			let _ = toggle_feature_flag(flag_id, enabled).await;
																			flags_resource.refetch();
																		});
																	}
																/>
															}
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
                                    <p class="font-semibold mb-1">"Ошибка загрузки флагов"</p>
                                    <p class="text-sm">{e.to_string()}</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Модалка создания/редактирования
            <Show when=move || show_create_modal.get() || editing_flag.get().is_some()>
                <FeatureFlagModal
                    flag=editing_flag.get()
                    on_close=move || {
                        set_show_create_modal.set(false);
                        set_editing_flag.set(None);
                    }
                    on_saved=move || {
                        set_show_create_modal.set(false);
                        set_editing_flag.set(None);
                        flags_resource.refetch();
                        set_toast.set(Some(ToastMessage {
                            message: "Флаг сохранён".to_string(),
                            variant: ToastVariant::Success,
                        }));
                    }
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

/// Вкладки страницы флагов.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FeaturesTab {
    /// Глобальные флаги.
    Global,
    /// Переопределения для тенантов.
    Overrides,
}

/// Строка таблицы функционального флага.
#[component]
fn FeatureFlagRow(
    flag: FeatureFlag,
    on_edit: impl Fn() + Send + Sync + 'static,
    on_toggle: impl Fn(Uuid, bool) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (Fn + Send + Sync)
    let on_edit = StoredValue::new(on_edit);
    let on_toggle = StoredValue::new(on_toggle);

    let scope_badge = match flag.scope {
        FeatureFlagScope::Global => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "🌍 Глобальный"),
        FeatureFlagScope::Tenant => ("bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300", "🏢 Тенант"),
        FeatureFlagScope::KillSwitch => ("bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300", "🚨 Kill switch"),
    };

    // ✅ Клонируем поля до view!
    let flag_id = flag.id;
    let is_enabled = flag.is_enabled;
    let key = flag.key.clone();
    let description = flag.description.clone();
    let has_overrides = flag.has_overrides;
    let override_count = flag.override_count;
    let updated_str = flag.updated_at.format("%d.%m.%Y %H:%M").to_string();

    view! {
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors">
            // Флаг
            <td class="px-4 py-3">
                <div class="flex items-start gap-3">
                    <div class=move || {
                        format!(
                            "w-10 h-10 rounded-lg flex items-center justify-center text-xl flex-shrink-0 {}",
                            if is_enabled {
                                "bg-green-100 dark:bg-green-900/30"
                            } else {
                                "bg-gray-100 dark:bg-gray-700"
                            }
                        )
                    }>
                        {if is_enabled { "✅" } else { "⏸️" }}
                    </div>
                    <div class="min-w-0">
                        <code class="text-sm font-mono font-semibold text-gray-900 dark:text-white block mb-0.5">
                            {key}
                        </code>
                        <p class="text-xs text-gray-500 dark:text-gray-400 line-clamp-1">
                            {description}
                        </p>
                    </div>
                </div>
            </td>

            // Область
            <td class="px-4 py-3">
                <span class=format!("inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {}", scope_badge.0)>
                    {scope_badge.1}
                </span>
            </td>

            // Статус (toggle)
            <td class="px-4 py-3">
                <label class="relative inline-flex items-center cursor-pointer">
                    <input
                        type="checkbox"
                        prop:checked=is_enabled
                        on:change=move |ev| {
                            on_toggle.with_value(|f| f(flag_id, event_target_checked(&ev)));
                        }
                        class="sr-only peer"
                    />
                    <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-green-600"></div>
                </label>
            </td>

            // Переопределения
            <td class="px-4 py-3">
                <Show when=move || has_overrides>
                    <button class="text-sm text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 font-medium">
                        {override_count} " переопределений"
                    </button>
                </Show>
                <Show when=move || !has_overrides>
                    <span class="text-sm text-gray-400 dark:text-gray-500">"—"</span>
                </Show>
            </td>

            // Обновлено
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                {updated_str}
            </td>

            // Действия
            <td class="px-4 py-3 text-right">
                <div class="flex items-center justify-end gap-2">
                    <button
                        on:click=move |_| on_edit.with_value(|f| f())
                        class="p-1.5 text-gray-500 hover:text-blue-600 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded transition-colors"
                        title="Редактировать"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                        </svg>
                    </button>
                    <button class="p-1.5 text-gray-500 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors" title="Удалить">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                        </svg>
                    </button>
                </div>
            </td>
        </tr>
    }
}

/// Модалка создания/редактирования флага.
#[component]
fn FeatureFlagModal(
    flag: Option<FeatureFlag>,
    on_close: impl Fn() + Send + Sync + 'static,
    on_saved: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_saved = StoredValue::new(on_saved);

    let is_editing = flag.is_some();
    let (key, set_key) = signal(flag.as_ref().map(|f| f.key.clone()).unwrap_or_default());
    let (description, set_description) = signal(flag.as_ref().map(|f| f.description.clone()).unwrap_or_default());
    let (scope, set_scope) = signal(flag.as_ref().map(|f| f.scope.clone()).unwrap_or(FeatureFlagScope::Global));
    let (is_enabled, set_is_enabled) = signal(flag.as_ref().map(|f| f.is_enabled).unwrap_or(true));
    let (rollout_percentage, set_rollout_percentage) = signal(flag.as_ref().map(|f| f.rollout_percentage).unwrap_or(100));

    let flag_id = flag.as_ref().map(|f| f.id);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let _ = save_feature_flag(SaveFeatureFlagPayload {
                id: flag_id,
                key: key.get(),
                description: description.get(),
                scope: scope.get(),
                is_enabled: is_enabled.get(),
                rollout_percentage: rollout_percentage.get(),
            }).await;
            on_saved.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">
                        {if is_editing { "Редактировать флаг" } else { "Создать функциональный флаг" }}
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
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Ключ флага (snake_case) *"</label>
                        <input
                            type="text"
                            prop:value=key
                            on:input=move |ev| set_key.set(event_target_value(&ev))
                            pattern="[a-z0-9_.]+"
                            disabled=is_editing
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm disabled:opacity-50"
                            placeholder="feature.new_ui.enabled"
                            required
                        />
                        <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">"Используется в коде для проверки доступности функции"</p>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Описание *"</label>
                        <textarea
                            prop:value=description
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                            rows="2"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            placeholder="Что контролирует этот флаг?"
                            required
                        ></textarea>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">"Область применения"</label>
                        <div class="grid grid-cols-3 gap-2">
                            <ScopeCard
                                icon="🌍"
                                label="Глобальный"
                                description="Для всех тенантов"
                                active=move || scope.get() == FeatureFlagScope::Global
                                on_click=move || set_scope.set(FeatureFlagScope::Global)
                            />
                            <ScopeCard
                                icon="🏢"
                                label="Тенант"
                                description="Переопределяется"
                                active=move || scope.get() == FeatureFlagScope::Tenant
                                on_click=move || set_scope.set(FeatureFlagScope::Tenant)
                            />
                            <ScopeCard
                                icon="🚨"
                                label="Kill switch"
                                description="Экстренное отключение"
                                active=move || scope.get() == FeatureFlagScope::KillSwitch
                                on_click=move || set_scope.set(FeatureFlagScope::KillSwitch)
                            />
                        </div>
                    </div>

                    <div class="flex items-start justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <div>
                            <div class="text-sm font-medium text-gray-900 dark:text-white">"Включён по умолчанию"</div>
                            <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">"Начальное состояние флага"</div>
                        </div>
                        <label class="relative inline-flex items-center cursor-pointer flex-shrink-0 ml-4">
                            <input
                                type="checkbox"
                                prop:checked=is_enabled
                                on:change=move |ev| set_is_enabled.set(event_target_checked(&ev))
                                class="sr-only peer"
                            />
                            <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-green-600"></div>
                        </label>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Rollout (% пользователей): " {rollout_percentage} "%"
                        </label>
                        <input
                            type="range"
                            min="0"
                            max="100"
                            step="5"
                            prop:value=rollout_percentage
                            on:input=move |ev| {
                                if let Ok(val) = event_target_value(&ev).parse() {
                                    set_rollout_percentage.set(val);
                                }
                            }
                            class="w-full"
                        />
                        <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1">
                            <span>"0% (выключено)"</span>
                            <span>"50% (половина)"</span>
                            <span>"100% (все)"</span>
                        </div>
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button type="submit" class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors">
                            {if is_editing { "Сохранить" } else { "Создать флаг" }}
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

/// Карточка области применения.
#[component]
fn ScopeCard(
    icon: &'static str,
    label: &'static str,
    description: &'static str,
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
            <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{description}</div>
        </button>
    }
}

/// Toast-уведомление.
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

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Область применения флага.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureFlagScope {
    /// Глобальный флаг.
    Global,
    /// Тенант-специфичный.
    Tenant,
    /// Kill switch (экстренное отключение).
    KillSwitch,
}

/// Функциональный флаг.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// ID флага.
    pub id: Uuid,
    /// Ключ (используется в коде).
    pub key: String,
    /// Описание.
    pub description: String,
    /// Область применения.
    pub scope: FeatureFlagScope,
    /// Включён ли.
    pub is_enabled: bool,
    /// Процент rollout (0-100).
    pub rollout_percentage: u8,
    /// Есть ли переопределения.
    pub has_overrides: bool,
    /// Количество переопределений.
    pub override_count: usize,
    /// Обновлено.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Payload для сохранения флага.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFeatureFlagPayload {
    /// ID (None для создания).
    pub id: Option<Uuid>,
    /// Ключ.
    pub key: String,
    /// Описание.
    pub description: String,
    /// Область применения.
    pub scope: FeatureFlagScope,
    /// Включён ли.
    pub is_enabled: bool,
    /// Процент rollout.
    pub rollout_percentage: u8,
}

/// Переопределение флага для тенанта.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantOverride {
    /// Идентификатор тенанта.
    pub tenant_id: Uuid,
    /// Название тенанта.
    pub tenant_name: String,
    /// Идентификатор флага.
    pub flag_id: Uuid,
    /// Включён ли флаг для тенанта.
    pub is_enabled: bool,
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

/// Получение всех функциональных флагов.
#[server(GetAllFeatureFlags, "/api")]
pub async fn fetch_all_feature_flags() -> Result<Vec<FeatureFlag>, ServerFnError> {
    // TODO: SELECT из feature_flags ORDER BY updated_at DESC
    // На этапе вёрстки возвращаем мок-данные.

    let now = chrono::Utc::now();

    let mock = vec![
        FeatureFlag {
            id: Uuid::new_v4(),
            key: "feature.new_ui.enabled".to_string(),
            description: "Новый интерфейс студенческого кабинета".to_string(),
            scope: FeatureFlagScope::Global,
            is_enabled: true,
            rollout_percentage: 100,
            has_overrides: true,
            override_count: 3,
            updated_at: now,
        },
        FeatureFlag {
            id: Uuid::new_v4(),
            key: "feature.ai_assistant.enabled".to_string(),
            description: "AI-ассистент для студентов".to_string(),
            scope: FeatureFlagScope::Tenant,
            is_enabled: false,
            rollout_percentage: 25,
            has_overrides: false,
            override_count: 0,
            updated_at: now - chrono::Duration::days(2),
        },
        FeatureFlag {
            id: Uuid::new_v4(),
            key: "feature.beta_certificates".to_string(),
            description: "Бета-версия генератора сертификатов".to_string(),
            scope: FeatureFlagScope::Global,
            is_enabled: true,
            rollout_percentage: 50,
            has_overrides: true,
            override_count: 1,
            updated_at: now - chrono::Duration::hours(5),
        },
        FeatureFlag {
            id: Uuid::new_v4(),
            key: "killswitch.video_upload".to_string(),
            description: "Экстренное отключение загрузки видео".to_string(),
            scope: FeatureFlagScope::KillSwitch,
            is_enabled: false,
            rollout_percentage: 0,
            has_overrides: false,
            override_count: 0,
            updated_at: now - chrono::Duration::days(10),
        },
        FeatureFlag {
            id: Uuid::new_v4(),
            key: "feature.offline_mode.enabled".to_string(),
            description: "Офлайн-режим для PWA".to_string(),
            scope: FeatureFlagScope::Global,
            is_enabled: true,
            rollout_percentage: 100,
            has_overrides: false,
            override_count: 0,
            updated_at: now - chrono::Duration::hours(1),
        },
    ];

    Ok(mock)
}

/// Получение переопределений для тенантов.
#[server(GetTenantOverrides, "/api")]
pub async fn fetch_tenant_overrides() -> Result<Vec<TenantOverride>, ServerFnError> {
    // TODO: SELECT из tenant_feature_overrides
    // На этапе вёрстки возвращаем пустой список.
    Ok(vec![])
}

/// Переключение состояния флага.
#[server(ToggleFeatureFlag, "/api")]
pub async fn toggle_feature_flag(_flag_id: Uuid, _enabled: bool) -> Result<(), ServerFnError> {
    // TODO: UPDATE feature_flags SET is_enabled = $1 WHERE id = $2
    // + NOTIFY feature_flags_changed
    Ok(())
}

/// Сохранение флага.
#[server(SaveFeatureFlag, "/api")]
pub async fn save_feature_flag(payload: SaveFeatureFlagPayload) -> Result<FeatureFlag, ServerFnError> {
    // TODO: INSERT/UPDATE feature_flags
    // На этапе вёрстки возвращаем мок-флаг из payload.

    let flag = FeatureFlag {
        id: payload.id.unwrap_or_else(Uuid::new_v4),
        key: payload.key,
        description: payload.description,
        scope: payload.scope,
        is_enabled: payload.is_enabled,
        rollout_percentage: payload.rollout_percentage,
        has_overrides: false,
        override_count: 0,
        updated_at: chrono::Utc::now(),
    };

    Ok(flag)
}