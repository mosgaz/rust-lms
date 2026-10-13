// crates/client/src/admin/pages/localization.rs
//! Управление переводами тенанта (Этап 18 i18n Infrastructure).
//!
//! CRUD для таблицы `tenant_translations`: кастомизация UI-строк тенантом,
//! переключение локалей, индикатор покрытия, импорт/экспорт, inline-редактирование.
//! Fallback-логика: tenant_locale → tenant_default → system_default (ru/en).
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ `Locale`, `TextDirection`, `TranslationKey` — локальные DTO:
//! они ещё не вынесены в `rust_lms_shared::models`. При интеграции с Этапом 18
//! — перенести в shared (по аналогии с Course/Node/Certificate).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Страница управления локализацией тенанта.
#[component]
pub fn AdminLocalization() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let keys_resource = Resource::new(|| (), |_| fetch_translation_keys());
    let locales_resource = Resource::new(|| (), |_| fetch_system_locales());
    let stats_resource = Resource::new(|| (), |_| fetch_localization_stats());

    // Сигналы состояния
    let (active_locale, set_active_locale) = signal(LocaleCode::Ru);
    let (fallback_locale, _set_fallback_locale) = signal(LocaleCode::En);
    let (search_query, set_search_query) = signal(String::new());
    let (filter_mode, set_filter_mode) = signal(FilterMode::All);
    let (category_filter, set_category_filter) = signal(None::<String>);
    let (show_add_key_modal, set_show_add_key_modal) = signal(false);
    let (show_import_modal, set_show_import_modal) = signal(false);
    let (editing_cell, set_editing_cell) = signal(None::<String>);
    let (toast, set_toast) = signal(None::<ToastMessage>);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        "Локализация"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        "Управление переводами интерфейса для вашего тенанта"
                    </p>
                </div>

                <div class="flex items-center gap-2">
                    <button
                        on:click=move |_| set_show_import_modal.set(true)
                        class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors flex items-center gap-2"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"></path>
                        </svg>
                        "Импорт"
                    </button>
                    <button
                        on:click=move |_| {
                            let locale_str = active_locale.get().as_str().to_string();
                            spawn_local(async move {
                                match export_translations(locale_str.clone()).await {
                                    Ok(json) => {
                                        download_json(&json, &format!("translations_{}.json", locale_str));
                                        set_toast.set(Some(ToastMessage {
                                            message: "Экспорт завершён".to_string(),
                                            variant: ToastVariant::Success,
                                        }));
                                    }
                                    Err(e) => {
                                        set_toast.set(Some(ToastMessage {
                                            message: format!("Ошибка: {}", e),
                                            variant: ToastVariant::Error,
                                        }));
                                    }
                                }
                            });
                        }
                        class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors flex items-center gap-2"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                        </svg>
                        "Экспорт"
                    </button>
                    <button
                        on:click=move |_| set_show_add_key_modal.set(true)
                        class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors flex items-center gap-2"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                        </svg>
                        "Добавить ключ"
                    </button>
                </div>
            </div>

            // Переключатель локалей
            <Suspense fallback=LocaleTabsSkeleton>
                {move || {
                    locales_resource.get().map(|result| {
                        match result {
                            Ok(locales) => view! {
                                <LocaleTabs
                                    locales=locales
                                    active=active_locale.into()
                                    on_change=move |locale| {
                                        set_active_locale.set(locale);
                                    }
                                />
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg text-sm mb-6">
                                    {e.to_string()}
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Статистика покрытия
            <Suspense fallback=StatsSkeleton>
                {move || {
                    stats_resource.get().map(|result| {
                        match result {
                            Ok(stats) => view! {
                                <CoverageStats
                                    stats=stats
                                    active_locale=active_locale.get()
                                />
                            }.into_any(),
                            Err(_) => view! { <div></div> }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Панель фильтров
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4 mb-6">
                <div class="flex flex-col lg:flex-row items-stretch lg:items-center gap-3">
                    // Поиск
                    <div class="relative flex-1">
                        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                        </svg>
                        <input
                            type="text"
                            placeholder="Поиск по ключу или значению..."
                            prop:value=search_query
                            on:input=move |ev| set_search_query.set(event_target_value(&ev))
                            class="w-full pl-10 pr-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm focus:ring-2 focus:ring-blue-500"
                        />
                    </div>

                    // Фильтр по категории
                    <select
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            set_category_filter.set(if val.is_empty() { None } else { Some(val) });
                        }
                        class="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                    >
                        <option value="">"Все категории"</option>
                        <option value="auth">"🔐 Аутентификация"</option>
                        <option value="student">"🎓 Студенческий интерфейс"</option>
                        <option value="admin">"⚙️ Админ-панель"</option>
                        <option value="course">"📚 Курсы"</option>
                        <option value="assessment">"📝 Тесты"</option>
                        <option value="certificate">"🏆 Сертификаты"</option>
                        <option value="notification">"🔔 Уведомления"</option>
                        <option value="error">"⚠️ Ошибки"</option>
                    </select>

                    // Фильтр по статусу перевода
                    <div class="flex bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
                        <FilterModeButton
                            label="Все"
                            active=move || filter_mode.get() == FilterMode::All
                            on_click=move || set_filter_mode.set(FilterMode::All)
                        />
                        <FilterModeButton
                            label="✓ Готово"
                            active=move || filter_mode.get() == FilterMode::Translated
                            on_click=move || set_filter_mode.set(FilterMode::Translated)
                        />
                        <FilterModeButton
                            label="⚠️ Нет перевода"
                            active=move || filter_mode.get() == FilterMode::Missing
                            on_click=move || set_filter_mode.set(FilterMode::Missing)
                        />
                        <FilterModeButton
                            label="🔧 Переопределено"
                            active=move || filter_mode.get() == FilterMode::Overridden
                            on_click=move || set_filter_mode.set(FilterMode::Overridden)
                        />
                    </div>
                </div>
            </div>

            // Таблица переводов
            <Suspense fallback=TableSkeleton>
                {move || {
                    keys_resource.get().map(|result| {
                        match result {
                            Ok(keys) => {
                                let filtered = keys.into_iter()
                                    .filter(|k| {
                                        let matches_search = search_query.get().is_empty() ||
                                            k.key.to_lowercase().contains(&search_query.get().to_lowercase()) ||
                                            k.values.values().any(|v| v.to_lowercase().contains(&search_query.get().to_lowercase()));

                                        let matches_category = category_filter.get().is_none() ||
                                            k.key.starts_with(&format!("{}.", category_filter.get().unwrap()));

                                        let matches_filter = match filter_mode.get() {
                                            FilterMode::All => true,
                                            FilterMode::Translated => k.values.contains_key(active_locale.get().as_str()),
                                            FilterMode::Missing => !k.values.contains_key(active_locale.get().as_str()),
                                            FilterMode::Overridden => k.is_tenant_override,
                                        };

                                        matches_search && matches_category && matches_filter
                                    })
                                    .collect::<Vec<_>>();

                                if filtered.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <div class="text-5xl mb-4">"🌐"</div>
                                            <p class="text-gray-500 dark:text-gray-400 text-lg mb-1">
                                                "Ключи не найдены"
                                            </p>
                                            <p class="text-gray-400 dark:text-gray-500 text-sm">
                                                "Измените параметры поиска или фильтры"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <TranslationsTable
                                            keys=filtered
                                            active_locale=active_locale.get()
                                            fallback_locale=fallback_locale.get()
                                            editing_cell=editing_cell.into()
                                            set_editing_cell=set_editing_cell
                                            on_save=move |key, value| {
                                                let locale_str = active_locale.get().as_str().to_string();
                                                spawn_local(async move {
                                                    match update_translation(key.clone(), locale_str, value).await {
                                                        Ok(_) => {
                                                            keys_resource.refetch();
                                                            stats_resource.refetch();
                                                            set_toast.set(Some(ToastMessage {
                                                                message: "Перевод сохранён".to_string(),
                                                                variant: ToastVariant::Success,
                                                            }));
                                                        }
                                                        Err(e) => {
                                                            set_toast.set(Some(ToastMessage {
                                                                message: format!("Ошибка: {}", e),
                                                                variant: ToastVariant::Error,
                                                            }));
                                                        }
                                                    }
                                                });
                                            }
                                            on_reset=move |key| {
                                                let locale_str = active_locale.get().as_str().to_string();
                                                spawn_local(async move {
                                                    let _ = reset_translation(key, locale_str).await;
                                                    keys_resource.refetch();
                                                    stats_resource.refetch();
                                                });
                                            }
                                            on_delete=move |key| {
                                                spawn_local(async move {
                                                    let _ = delete_translation_key(key).await;
                                                    keys_resource.refetch();
                                                    stats_resource.refetch();
                                                });
                                            }
                                        />
                                    }.into_any()
                                }
                            }
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold mb-1">"Ошибка загрузки ключей"</p>
                                    <p class="text-sm">{e.to_string()}</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Модалка добавления ключа
            <Show when=move || show_add_key_modal.get()>
                <AddKeyModal
                    on_close=move || set_show_add_key_modal.set(false)
                    on_added=move || {
                        set_show_add_key_modal.set(false);
                        keys_resource.refetch();
                    }
                />
            </Show>

            // Модалка импорта
            <Show when=move || show_import_modal.get()>
                <ImportModal
                    on_close=move || set_show_import_modal.set(false)
                    on_imported=move || {
                        set_show_import_modal.set(false);
                        keys_resource.refetch();
                        stats_resource.refetch();
                        set_toast.set(Some(ToastMessage {
                            message: "Импорт завершён".to_string(),
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

/// Код локали (упрощённая версия).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum LocaleCode {
    /// Русский.
    Ru,
    /// Английский.
    En,
    /// Арабский (RTL).
    Ar,
    /// Другой.
    Other,
}

impl LocaleCode {
    /// Возвращает строковое представление.
    pub fn as_str(&self) -> &'static str {
        match self {
            LocaleCode::Ru => "ru",
            LocaleCode::En => "en",
            LocaleCode::Ar => "ar",
            LocaleCode::Other => "other",
        }
    }

    /// Возвращает отображаемое название.
    pub fn display_name(&self) -> &'static str {
        match self {
            LocaleCode::Ru => "Русский",
            LocaleCode::En => "English",
            LocaleCode::Ar => "العربية",
            LocaleCode::Other => "Другой",
        }
    }

    /// Возвращает флаг.
    pub fn flag(&self) -> &'static str {
        match self {
            LocaleCode::Ru => "🇷🇺",
            LocaleCode::En => "🇬🇧",
            LocaleCode::Ar => "🇸🇦",
            LocaleCode::Other => "🌐",
        }
    }

    /// Направление текста.
    pub fn direction(&self) -> TextDirection {
        match self {
            LocaleCode::Ar => TextDirection::Rtl,
            _ => TextDirection::Ltr,
        }
    }
}

/// Направление текста.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TextDirection {
    /// Слева направо.
    Ltr,
    /// Справа налево.
    Rtl,
}

/// Локаль (для списка локалей на странице).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Locale {
    /// Код локали ("ru", "en", "ar", ...).
    pub code: String,
    /// Отображаемое название.
    pub name: String,
    /// Флаг.
    pub flag: String,
    /// Направление текста.
    pub direction: TextDirection,
}

/// Ключ локализации.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranslationKey {
    /// Ключ ("auth.login.title").
    pub key: String,
    /// Значения по локалям: { "ru": "Вход", "en": "Login" }.
    pub values: std::collections::HashMap<String, String>,
    /// Был ли ключ переопределён тенантом.
    pub is_tenant_override: bool,
}

/// Режим фильтрации переводов.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterMode {
    /// Все ключи.
    All,
    /// Только с переводом для текущей локали.
    Translated,
    /// Только без перевода.
    Missing,
    /// Только переопределённые тенантом.
    Overridden,
}

/// Кнопка режима фильтра.
#[component]
fn FilterModeButton(
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
                format!(
                    "px-3 py-1 text-xs font-medium rounded transition-all whitespace-nowrap {}",
                    if active.with_value(|f| f()) {
                        "bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm"
                    } else {
                        "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                    }
                )
            }
        >
            {label}
        </button>
    }
}

/// Вкладки переключения локалей.
#[component]
fn LocaleTabs(
    locales: Vec<Locale>,
    active: Signal<LocaleCode>,
    on_change: impl Fn(LocaleCode) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_change = StoredValue::new(on_change);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-2 mb-6 flex flex-wrap gap-2">
            {locales.into_iter().map(|locale| {
                let code = match locale.code.as_str() {
                    "ru" => LocaleCode::Ru,
                    "en" => LocaleCode::En,
                    "ar" => LocaleCode::Ar,
                    _ => LocaleCode::Other,
                };
                let flag = locale.flag.clone();
                let name = locale.name.clone();
                let code_str = locale.code.to_uppercase();
                let direction = locale.direction;
                view! {
                    <button
                        on:click=move |_| on_change.with_value(|f| f(code))
                        class=move || {
                            format!(
                                "flex items-center gap-2 px-4 py-2 rounded-lg font-medium transition-all {}",
                                if active.get() == code {
                                    "bg-blue-600 text-white shadow-sm"
                                } else {
                                    "text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                                }
                            )
                        }
                    >
                        <span class="text-lg">{flag}</span>
                        <span>{name}</span>
                        <span class=move || {
                            format!(
                                "text-xs px-1.5 py-0.5 rounded-full {}",
                                if active.get() == code {
                                    "bg-white/20 text-white"
                                } else {
                                    "bg-gray-200 dark:bg-gray-600 text-gray-600 dark:text-gray-300"
                                }
                            )
                        }>
                            {code_str}
                        </span>
                        <Show when=move || direction == TextDirection::Rtl>
                            <span class="text-xs px-1.5 py-0.5 bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 rounded">
                                "RTL"
                            </span>
                        </Show>
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Статистика покрытия переводами.
#[component]
fn CoverageStats(stats: LocalizationStats, active_locale: LocaleCode) -> impl IntoView {
    let coverage_pct = if stats.total_keys > 0 {
        (stats.translated_keys as f64 / stats.total_keys as f64 * 100.0).round() as u32
    } else {
        0
    };

    let bar_color = if coverage_pct >= 90 {
        "bg-green-500"
    } else if coverage_pct >= 60 {
        "bg-yellow-500"
    } else {
        "bg-red-500"
    };

    let locale_label = active_locale.display_name();
    let total_keys = stats.total_keys;
    let translated_keys = stats.translated_keys;
    let overridden_keys = stats.overridden_keys;

    view! {
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                    "Покрытие (" {locale_label} ")"
                </div>
                <div class="flex items-baseline gap-2 mb-2">
                    <div class="text-3xl font-bold text-gray-900 dark:text-white">{coverage_pct} "%" </div>
                </div>
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                    <div
                        class=format!("{} h-2 rounded-full transition-all", bar_color)
                        style:width=format!("{}%", coverage_pct)
                    ></div>
                </div>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                    "Всего ключей"
                </div>
                <div class="text-3xl font-bold text-gray-900 dark:text-white">
                    {total_keys}
                </div>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                    "Переведено"
                </div>
                <div class="text-3xl font-bold text-green-600 dark:text-green-400">
                    {translated_keys}
                </div>
            </div>

            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
                <div class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                    "Переопределено тенантом"
                </div>
                <div class="text-3xl font-bold text-blue-600 dark:text-blue-400">
                    {overridden_keys}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                    "Кастомные строки"
                </div>
            </div>
        </div>
    }
}

/// Таблица переводов с inline-редактированием.
#[component]
fn TranslationsTable(
    keys: Vec<TranslationKey>,
    active_locale: LocaleCode,
    fallback_locale: LocaleCode,
    editing_cell: Signal<Option<String>>,
    set_editing_cell: WriteSignal<Option<String>>,
    on_save: impl Fn(String, String) + Send + Sync + 'static,
    on_reset: impl Fn(String) + Send + Sync + 'static,
    on_delete: impl Fn(String) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_save = StoredValue::new(on_save);
    let on_reset = StoredValue::new(on_reset);
    let on_delete = StoredValue::new(on_delete);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                        <tr>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase w-1/3">
                                "Ключ"
                            </th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase w-1/3">
                                {active_locale.flag()} " " {active_locale.display_name()}
                            </th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase w-1/4">
                                {fallback_locale.flag()} " " {fallback_locale.display_name()} " (fallback)"
                            </th>
                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase w-24">
                                "Действия"
                            </th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                        {keys.into_iter().map(|key| {
                            view! {
                                <TranslationRow
                                    key_data=key
                                    active_locale=active_locale
                                    fallback_locale=fallback_locale
                                    editing_cell=editing_cell
                                    set_editing_cell=set_editing_cell
                                    on_save=on_save
                                    on_reset=on_reset
                                    on_delete=on_delete
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// Строка таблицы перевода с inline-редактированием.
#[component]
fn TranslationRow(
    key_data: TranslationKey,
    active_locale: LocaleCode,
    fallback_locale: LocaleCode,
    editing_cell: Signal<Option<String>>,
    set_editing_cell: WriteSignal<Option<String>>,
    on_save: StoredValue<impl Fn(String, String) + Send + Sync + 'static>,
    on_reset: StoredValue<impl Fn(String) + Send + Sync + 'static>,
    on_delete: StoredValue<impl Fn(String) + Send + Sync + 'static>,
) -> impl IntoView {
    // ✅ Клонируем поля до замыканий
    let key_for_check = key_data.key.clone();
    let key_display = key_data.key.clone();

    let is_tenant_override = key_data.is_tenant_override;
    let current_value = key_data.values.get(active_locale.as_str()).cloned().unwrap_or_default();
    let fallback_value = key_data
        .values
        .get(fallback_locale.as_str())
        .cloned()
        .unwrap_or_else(|| "—".to_string());
    let is_rtl = active_locale.direction() == TextDirection::Rtl;

    let current_value_for_signal = current_value.clone();
    let current_value_for_edit = current_value.clone();
    let (edit_value, set_edit_value) = signal(current_value_for_edit);

    // ✅ Предвычисляем класс кнопки заранее — иначе замыкание становится FnOnce.
    // Оборачиваем в StoredValue, чтобы `class=move || button_class.get()`
    // не перемещал String.
    let button_class = StoredValue::new(if current_value.is_empty() {
        "w-full text-left p-2 rounded border border-transparent hover:border-blue-300 dark:hover:border-blue-600 transition-colors min-h-[40px] text-sm text-gray-400 dark:text-gray-500 italic".to_string()
    } else {
        "w-full text-left p-2 rounded border border-transparent hover:border-blue-300 dark:hover:border-blue-600 transition-colors min-h-[40px] text-sm text-gray-900 dark:text-white".to_string()
    });
    let has_value = !current_value.is_empty();
    let current_value_for_button = current_value.clone();

    // ✅ StoredValue: делает closures Copy → Fn + Send + Sync
    let is_editing = {
        let key = key_for_check.clone();
        StoredValue::new(move || {
            editing_cell.get().as_deref() == Some(key.as_str())
        })
    };

    let start_editing = {
        let key = key_data.key.clone();
        let current = current_value_for_signal.clone();
        StoredValue::new(move || {
            set_edit_value.set(current.clone());
            set_editing_cell.set(Some(key.clone()));
        })
    };

    let save_edit = {
        let key = key_data.key.clone();
        StoredValue::new(move || {
            let value = edit_value.get();
            on_save.with_value(|f| f(key.clone(), value));
            set_editing_cell.set(None);
        })
    };

    let cancel_edit = StoredValue::new(move || {
        set_editing_cell.set(None);
    });

    let handle_reset = {
        let key = key_data.key.clone();
        StoredValue::new(move || {
            on_reset.with_value(|f| f(key.clone()));
        })
    };

    let handle_delete = {
        let key = key_data.key.clone();
        StoredValue::new(move || {
            on_delete.with_value(|f| f(key.clone()));
        })
    };

    let row_bg = if is_tenant_override {
        "bg-blue-50/30 dark:bg-blue-900/10"
    } else if current_value.is_empty() {
        "bg-yellow-50/30 dark:bg-yellow-900/10"
    } else {
        ""
    };

    view! {
        <tr class=format!("{} hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors", row_bg)>
            // Ключ
            <td class="px-4 py-3">
                <div class="flex items-start gap-2">
                    <div class="flex-1 min-w-0">
                        <code class="text-xs font-mono text-gray-700 dark:text-gray-300 break-all">
                            {key_display}
                        </code>
                        <Show when=move || is_tenant_override>
                            <div class="mt-1">
                                <span class="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded">
                                    "🔧 Переопределено"
                                </span>
                            </div>
                        </Show>
                        <Show when=move || !has_value>
                            <div class="mt-1">
                                <span class="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 rounded">
                                    "⚠️ Нет перевода"
                                </span>
                            </div>
                        </Show>
                    </div>
                </div>
            </td>

            // Текущая локаль (редактируемая)
            <td class="px-4 py-3">
                <Show when=move || !is_editing.with_value(|f| f())>
                    <button
                        on:click=move |_| start_editing.with_value(|f| f())
                        class=move || button_class.with_value(|s| s.clone())
                        dir=if is_rtl { "rtl" } else { "ltr" }
                    >
                        {if current_value_for_button.is_empty() {
                            "Нажмите для добавления...".to_string()
                        } else {
                            current_value_for_button.clone()
                        }}
                    </button>
                </Show>

                <Show when=move || is_editing.with_value(|f| f())>
                    <div class="space-y-2">
                        <textarea
                            prop:value=edit_value
                            on:input=move |ev| set_edit_value.set(event_target_value(&ev))
                            rows="2"
                            class="w-full px-2 py-1.5 text-sm border-2 border-blue-500 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none"
                            dir=if is_rtl { "rtl" } else { "ltr" }
                        ></textarea>
                        <div class="flex items-center gap-1">
                            <button
                                on:click=move |_| save_edit.with_value(|f| f())
                                class="px-2 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                            >
                                "✓ Сохранить"
                            </button>
                            <button
                                on:click=move |_| cancel_edit.with_value(|f| f())
                                class="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                            >
                                "✗ Отмена"
                            </button>
                        </div>
                    </div>
                </Show>
            </td>

            // Fallback значение (только для чтения)
            <td class="px-4 py-3">
                <div class="text-sm text-gray-500 dark:text-gray-400 p-2 bg-gray-50 dark:bg-gray-900/30 rounded min-h-[40px] italic">
                    {fallback_value}
                </div>
            </td>

            // Действия
            <td class="px-4 py-3 text-right">
                <div class="flex items-center justify-end gap-1">
                    <Show when=move || is_tenant_override>
                        <button
                            on:click=move |_| handle_reset.with_value(|f| f())
                            class="p-1.5 text-gray-500 hover:text-orange-600 dark:hover:text-orange-400 hover:bg-orange-50 dark:hover:bg-orange-900/20 rounded transition-colors"
                            title="Сбросить к системному значению"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                            </svg>
                        </button>
                    </Show>
                    <button
                        on:click=move |_| handle_delete.with_value(|f| f())
                        class="p-1.5 text-gray-500 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
                        title="Удалить ключ"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                        </svg>
                    </button>
                </div>
            </td>
        </tr>
    }
}

/// Модалка добавления нового ключа.
#[component]
fn AddKeyModal(
    on_close: impl Fn() + Send + Sync + 'static,
    on_added: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_added = StoredValue::new(on_added);

    let (key, set_key) = signal(String::new());
    let (category, set_category) = signal("admin".to_string());
    let (values, set_values) = signal(std::collections::HashMap::<String, String>::new());

    let full_key = move || format!("{}.{}", category.get(), key.get());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let payload = AddKeyPayload {
            key: full_key(),
            values: values.get().clone(),
        };
        spawn_local(async move {
            let _ = add_translation_key(payload).await;
            on_added.with_value(|f| f());
        });
    };

    let update_value_ru = move |value: String| {
        let mut current = values.get();
        current.insert("ru".to_string(), value);
        set_values.set(current);
    };
    let update_value_en = move |value: String| {
        let mut current = values.get();
        current.insert("en".to_string(), value);
        set_values.set(current);
    };
    let update_value_ar = move |value: String| {
        let mut current = values.get();
        current.insert("ar".to_string(), value);
        set_values.set(current);
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">"Добавить ключ локализации"</h3>
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
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Категория"</label>
                        <select
                            on:change=move |ev| set_category.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        >
                            <option value="auth">"🔐 auth — Аутентификация"</option>
                            <option value="student">"🎓 student — Студенческий интерфейс"</option>
                            <option value="admin">"⚙️ admin — Админ-панель"</option>
                            <option value="course">"📚 course — Курсы"</option>
                            <option value="assessment">"📝 assessment — Тесты"</option>
                            <option value="certificate">"🏆 certificate — Сертификаты"</option>
                            <option value="notification">"🔔 notification — Уведомления"</option>
                            <option value="error">"⚠️ error — Ошибки"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Имя ключа (snake_case) *"</label>
                        <input
                            type="text"
                            prop:value=key
                            on:input=move |ev| set_key.set(event_target_value(&ev))
                            pattern="[a-z0-9_]+"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                            placeholder="save_button"
                            required
                        />
                        <div class="mt-2 p-2 bg-gray-50 dark:bg-gray-900/50 rounded text-xs font-mono text-gray-600 dark:text-gray-400">
                            "Итоговый ключ: " <span class="text-blue-600 dark:text-blue-400 font-semibold">{full_key()}</span>
                        </div>
                    </div>

                    <div class="space-y-3">
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Значения по локалям"</label>

                        <div>
                            <label class="text-xs text-gray-600 dark:text-gray-400 mb-1 block">"🇷🇺 Русский *"</label>
                            <input
                                type="text"
                                on:input=move |ev| update_value_ru(event_target_value(&ev))
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                placeholder="Сохранить"
                                required
                            />
                        </div>

                        <div>
                            <label class="text-xs text-gray-600 dark:text-gray-400 mb-1 block">"🇬🇧 English *"</label>
                            <input
                                type="text"
                                on:input=move |ev| update_value_en(event_target_value(&ev))
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                placeholder="Save"
                                required
                            />
                        </div>

                        <div>
                            <label class="text-xs text-gray-600 dark:text-gray-400 mb-1 block">"🇸🇦 العربية (опционально, RTL)"</label>
                            <input
                                type="text"
                                dir="rtl"
                                on:input=move |ev| update_value_ar(event_target_value(&ev))
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                placeholder="حفظ"
                            />
                        </div>
                    </div>

                    <div class="p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg text-sm text-blue-800 dark:text-blue-300">
                        <p class="font-medium mb-1">"💡 Совет"</p>
                        <p>
                            "Используйте snake_case для ключей. Для параметров Fluent используйте синтаксис: "
                            <code class="px-1 bg-blue-100 dark:bg-blue-900/40 rounded">"Привет, $name!"</code>
                        </p>
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                        >
                            "Добавить ключ"
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

/// Модалка импорта переводов.
#[component]
fn ImportModal(
    on_close: impl Fn() + Send + Sync + 'static,
    on_imported: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_imported = StoredValue::new(on_imported);

    let (format, set_format) = signal(ImportFormat::Json);
    let (locale, set_locale) = signal(LocaleCode::Ru);
    let (file_content, set_file_content) = signal(String::new());
    let (mode, set_mode) = signal(ImportMode::Merge);

    let handle_file = move |ev: web_sys::Event| {
        // ✅ JsCast в scope
        if let Some(input) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
            if let Some(files) = input.files() {
                if let Some(file) = files.item(0) {
                    let reader = web_sys::FileReader::new().unwrap();
                    let reader_clone = reader.clone();
                    let on_load = Closure::<dyn FnMut(_)>::new(move |_: web_sys::Event| {
                        if let Ok(result) = reader_clone.result() {
                            if let Some(text) = result.as_string() {
                                set_file_content.set(text);
                            }
                        }
                    });
                    reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
                    on_load.forget();
                    let _ = reader.read_as_text(&file);
                }
            }
        }
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let payload = ImportPayload {
            format: format.get(),
            locale: locale.get().as_str().to_string(),
            content: file_content.get(),
            mode: mode.get(),
        };
        spawn_local(async move {
            let _ = import_translations(payload).await;
            on_imported.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-lg w-full">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white">"Импорт переводов"</h3>
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
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">"Формат файла"</label>
                        <div class="grid grid-cols-2 gap-2">
                            <label class="cursor-pointer">
                                <input
                                    type="radio"
                                    name="format"
                                    prop:checked=move || format.get() == ImportFormat::Json
                                    on:change=move |_| set_format.set(ImportFormat::Json)
                                    class="sr-only peer"
                                />
                                <div class=move || {
                                    format!(
                                        "p-3 border-2 rounded-lg text-center transition-all {}",
                                        if format.get() == ImportFormat::Json {
                                            "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                                        } else {
                                            "border-gray-200 dark:border-gray-700"
                                        }
                                    )
                                }>
                                    <div class="text-xl mb-1">"📋"</div>
                                    <div class="text-sm font-medium">"JSON"</div>
                                </div>
                            </label>
                            <label class="cursor-pointer">
                                <input
                                    type="radio"
                                    name="format"
                                    prop:checked=move || format.get() == ImportFormat::Fluent
                                    on:change=move |_| set_format.set(ImportFormat::Fluent)
                                    class="sr-only peer"
                                />
                                <div class=move || {
                                    format!(
                                        "p-3 border-2 rounded-lg text-center transition-all {}",
                                        if format.get() == ImportFormat::Fluent {
                                            "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                                        } else {
                                            "border-gray-200 dark:border-gray-700"
                                        }
                                    )
                                }>
                                    <div class="text-xl mb-1">"🌐"</div>
                                    <div class="text-sm font-medium">"Fluent (.ftl)"</div>
                                </div>
                            </label>
                        </div>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Целевая локаль"</label>
                        <select
                            on:change=move |ev| {
                                set_locale.set(match event_target_value(&ev).as_str() {
                                    "en" => LocaleCode::En,
                                    "ar" => LocaleCode::Ar,
                                    _ => LocaleCode::Ru,
                                });
                            }
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        >
                            <option value="ru">"🇷🇺 Русский"</option>
                            <option value="en">"🇬🇧 English"</option>
                            <option value="ar">"🇸🇦 العربية"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">"Режим импорта"</label>
                        <div class="space-y-2">
                            <label class="flex items-start gap-2 p-3 border border-gray-200 dark:border-gray-700 rounded-lg cursor-pointer hover:border-blue-300 dark:hover:border-blue-600 transition-colors">
                                <input
                                    type="radio"
                                    name="mode"
                                    prop:checked=move || mode.get() == ImportMode::Merge
                                    on:change=move |_| set_mode.set(ImportMode::Merge)
                                    class="mt-0.5"
                                />
                                <div>
                                    <div class="text-sm font-medium text-gray-900 dark:text-white">"Объединить (Merge)"</div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400">"Добавить новые, обновить существующие"</div>
                                </div>
                            </label>
                            <label class="flex items-start gap-2 p-3 border border-gray-200 dark:border-gray-700 rounded-lg cursor-pointer hover:border-blue-300 dark:hover:border-blue-600 transition-colors">
                                <input
                                    type="radio"
                                    name="mode"
                                    prop:checked=move || mode.get() == ImportMode::Replace
                                    on:change=move |_| set_mode.set(ImportMode::Replace)
                                    class="mt-0.5"
                                />
                                <div>
                                    <div class="text-sm font-medium text-gray-900 dark:text-white">"Заменить (Replace)"</div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400">"Полностью заменить все переводы"</div>
                                </div>
                            </label>
                        </div>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Файл с переводами *"</label>
                        <input
                            type="file"
                            accept=".json,.ftl"
                            on:change=handle_file
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                            required
                        />
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                        >
                            "Импортировать"
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
fn LocaleTabsSkeleton() -> impl IntoView {
    view! {
        <div class="h-14 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse mb-6"></div>
    }
}

#[component]
fn StatsSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6 animate-pulse">
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-28 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

#[component]
fn TableSkeleton() -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden animate-pulse">
            <div class="h-12 bg-gray-200 dark:bg-gray-700"></div>
            <div class="divide-y divide-gray-200 dark:divide-gray-700">
                {(0..8).map(|_| {
                    view! { <div class="h-20 bg-gray-50 dark:bg-gray-800/50"></div> }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Режим импорта.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportMode {
    /// Объединить с существующими.
    Merge,
    /// Полностью заменить.
    Replace,
}

/// Формат файла импорта.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportFormat {
    /// JSON.
    Json,
    /// Fluent (.ftl).
    Fluent,
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

/// Payload для добавления ключа.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddKeyPayload {
    /// Полный ключ (category.name).
    pub key: String,
    /// Значения по локалям.
    pub values: std::collections::HashMap<String, String>,
}

/// Payload для импорта.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPayload {
    /// Формат файла.
    pub format: ImportFormat,
    /// Целевая локаль.
    pub locale: String,
    /// Содержимое файла.
    pub content: String,
    /// Режим импорта.
    pub mode: ImportMode,
}

/// Статистика локализации.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationStats {
    /// Общее количество ключей.
    pub total_keys: usize,
    /// Количество переведённых ключей для активной локали.
    pub translated_keys: usize,
    /// Количество переопределённых тенантом ключей.
    pub overridden_keys: usize,
}

// ----------------------------------------------------------------------------
// Вспомогательные функции
// ----------------------------------------------------------------------------

/// Скачивание JSON как файла.
fn download_json(content: &str, filename: &str) {
    if let Some(window) = web_sys::window() {
        let document = window.document().unwrap();
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("application/json");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(
            &js_sys::Array::of1(&content.into()),
            &options,
        )
        .unwrap();
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
        let link = document.create_element("a").unwrap();
        link.set_attribute("href", &url).unwrap();
        link.set_attribute("download", filename).unwrap();
        link.dyn_into::<web_sys::HtmlElement>().unwrap().click();
        web_sys::Url::revoke_object_url(&url).unwrap();
    }
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

/// Получение списка всех ключей локализации.
#[server(GetTranslationKeys, "/api")]
pub async fn fetch_translation_keys() -> Result<Vec<TranslationKey>, ServerFnError> {
    // TODO: Интеграция с таблицей tenant_translations + системными переводами
    // На этапе вёрстки возвращаем 6 мок-ключей.
    use std::collections::HashMap;

    let mut values1 = HashMap::new();
    values1.insert("ru".to_string(), "Вход в систему".to_string());
    values1.insert("en".to_string(), "Sign in".to_string());
    values1.insert("ar".to_string(), "تسجيل الدخول".to_string());

    let mut values2 = HashMap::new();
    values2.insert("ru".to_string(), "Сохранить".to_string());
    values2.insert("en".to_string(), "Save".to_string());

    let mut values3 = HashMap::new();
    values3.insert("ru".to_string(), "Мои курсы".to_string());
    values3.insert("en".to_string(), "My courses".to_string());

    let mut values4 = HashMap::new();
    values4.insert("ru".to_string(), "Завершить курс".to_string());

    let mock = vec![
        TranslationKey {
            key: "auth.login.title".to_string(),
            values: values1,
            is_tenant_override: false,
        },
        TranslationKey {
            key: "admin.save_button".to_string(),
            values: values2,
            is_tenant_override: true,
        },
        TranslationKey {
            key: "student.courses.title".to_string(),
            values: values3,
            is_tenant_override: false,
        },
        TranslationKey {
            key: "course.complete_button".to_string(),
            values: values4,
            is_tenant_override: false,
        },
        TranslationKey {
            key: "assessment.submit_button".to_string(),
            values: HashMap::new(),
            is_tenant_override: false,
        },
        TranslationKey {
            key: "certificate.verify_link".to_string(),
            values: HashMap::new(),
            is_tenant_override: true,
        },
    ];

    Ok(mock)
}

/// Получение списка поддерживаемых локалей.
#[server(GetSystemLocales, "/api")]
pub async fn fetch_system_locales() -> Result<Vec<Locale>, ServerFnError> {
    // TODO: Из конфигурации системы
    // На этапе вёрстки возвращаем 3 локали.
    Ok(vec![
        Locale {
            code: "ru".to_string(),
            name: "Русский".to_string(),
            flag: "🇷🇺".to_string(),
            direction: TextDirection::Ltr,
        },
        Locale {
            code: "en".to_string(),
            name: "English".to_string(),
            flag: "🇬🇧".to_string(),
            direction: TextDirection::Ltr,
        },
        Locale {
            code: "ar".to_string(),
            name: "العربية".to_string(),
            flag: "🇸🇦".to_string(),
            direction: TextDirection::Rtl,
        },
    ])
}

/// Получение статистики локализации.
#[server(GetLocalizationStats, "/api")]
pub async fn fetch_localization_stats() -> Result<LocalizationStats, ServerFnError> {
    // TODO: SQL агрегация
    // На этапе вёрстки возвращаем мок-статистику.
    Ok(LocalizationStats {
        total_keys: 342,
        translated_keys: 289,
        overridden_keys: 12,
    })
}

/// Обновление перевода.
#[server(UpdateTranslation, "/api")]
pub async fn update_translation(
    _key: String,
    _locale: String,
    _value: String,
) -> Result<(), ServerFnError> {
    // TODO: UPSERT в tenant_translations
    Ok(())
}

/// Добавление нового ключа.
#[server(AddTranslationKey, "/api")]
pub async fn add_translation_key(_payload: AddKeyPayload) -> Result<(), ServerFnError> {
    // TODO: Вставка ключа + значений для всех локалей
    Ok(())
}

/// Сброс перевода к системному значению.
#[server(ResetTranslation, "/api")]
pub async fn reset_translation(_key: String, _locale: String) -> Result<(), ServerFnError> {
    // TODO: DELETE из tenant_translations (чтобы сработал fallback)
    Ok(())
}

/// Удаление ключа.
#[server(DeleteTranslationKey, "/api")]
pub async fn delete_translation_key(_key: String) -> Result<(), ServerFnError> {
    // TODO: DELETE из tenant_translations WHERE key = $1
    Ok(())
}

/// Экспорт переводов.
#[server(ExportTranslations, "/api")]
pub async fn export_translations(_locale: String) -> Result<String, ServerFnError> {
    // TODO: SELECT key, value FROM tenant_translations WHERE locale = $1
    // На этапе вёрстки возвращаем пустой JSON.
    Ok("{}".to_string())
}

/// Импорт переводов.
#[server(ImportTranslations, "/api")]
pub async fn import_translations(_payload: ImportPayload) -> Result<(), ServerFnError> {
    // TODO: Парсинг JSON/.ftl + массовый UPSERT
    Ok(())
}