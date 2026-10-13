// crates/client/src/admin/pages/settings.rs
//! Общие настройки системы.
//!
//! Управление SMTP, JWT, feature flags, локализацией и другими параметрами.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функция возвращает мок-данные.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Страница настроек системы.
#[component]
pub fn AdminSettingList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let settings_resource = Resource::new(|| (), |_| fetch_system_settings());

    view! {
        <div class="container mx-auto px-4 py-8 max-w-4xl">
            <h1 class="text-3xl font-bold mb-2 text-gray-900 dark:text-white">
                // i18n: t!("admin.settings.title")
                "Настройки"
            </h1>
            <p class="text-gray-600 dark:text-gray-400 mb-8">
                // i18n: t!("admin.settings.subtitle")
                "Конфигурация платформы"
            </p>

            <Suspense fallback=SettingsSkeleton>
                {move || {
                    settings_resource.get().map(|result| {
                        match result {
                            Ok(settings) => view! {
                                <div class="space-y-6">
                                    <SmtpSettingsSection settings=settings.smtp.clone() />
                                    <JwtSettingsSection settings=settings.jwt.clone() />
                                    <LocalizationSettingsSection settings=settings.localization.clone() />
                                </div>
                            }.into_any(),
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

/// Секция настроек SMTP.
#[component]
fn SmtpSettingsSection(settings: SmtpSettings) -> impl IntoView {
    let (host, set_host) = signal(settings.host);
    let (port, set_port) = signal(settings.port);
    let (username, set_username) = signal(settings.username);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">
                "SMTP (Email)"
            </h2>
            <div class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "SMTP Host"
                    </label>
                    <input
                        type="text"
                        prop:value=host
                        on:input=move |ev| set_host.set(event_target_value(&ev))
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Port"
                    </label>
                    <input
                        type="number"
                        prop:value=port
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse() {
                                set_port.set(val);
                            }
                        }
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Username"
                    </label>
                    <input
                        type="text"
                        prop:value=username
                        on:input=move |ev| set_username.set(event_target_value(&ev))
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                    />
                </div>
                <button class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors">
                    "Сохранить"
                </button>
            </div>
        </div>
    }
}

/// Секция настроек JWT.
#[component]
fn JwtSettingsSection(settings: JwtSettings) -> impl IntoView {
    let (access_ttl, set_access_ttl) = signal(settings.access_ttl_minutes);
    let (refresh_ttl, set_refresh_ttl) = signal(settings.refresh_ttl_minutes);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">
                "JWT токены"
            </h2>
            <div class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Access Token TTL (минуты)"
                    </label>
                    <input
                        type="number"
                        prop:value=access_ttl
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse() {
                                set_access_ttl.set(val);
                            }
                        }
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Refresh Token TTL (минуты)"
                    </label>
                    <input
                        type="number"
                        prop:value=refresh_ttl
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse() {
                                set_refresh_ttl.set(val);
                            }
                        }
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                    />
                </div>
                <button class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors">
                    "Сохранить"
                </button>
            </div>
        </div>
    }
}

/// Секция настроек локализации.
#[component]
fn LocalizationSettingsSection(settings: LocalizationSettings) -> impl IntoView {
    let (default_locale, set_default_locale) = signal(settings.default_locale);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">
                "Локализация"
            </h2>
            <div class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        "Язык по умолчанию"
                    </label>
                    <select
                        prop:value=default_locale
                        on:change=move |ev| set_default_locale.set(event_target_value(&ev))
                        class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                    >
                        <option value="ru">"Русский"</option>
                        <option value="en">"English"</option>
                        <option value="ar">"العربية"</option>
                    </select>
                </div>
                <button class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors">
                    "Сохранить"
                </button>
            </div>
        </div>
    }
}

#[component]
fn SettingsSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-6 animate-pulse">
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Общие настройки системы (локальный DTO для страницы `/admin/settings`).
///
/// ⚠️ Хранятся вне модели предметной области — конфигурация платформы,
/// не привязанная к тенанту. При подключении БД — источник может быть:
/// - отдельная таблица `system_settings` с одной строкой;
/// - файл конфигурации;
/// - переменные окружения.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    /// Настройки SMTP.
    pub smtp: SmtpSettings,
    /// Настройки JWT.
    pub jwt: JwtSettings,
    /// Настройки локализации.
    pub localization: LocalizationSettings,
}

/// Настройки SMTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpSettings {
    /// Хост SMTP-сервера.
    pub host: String,
    /// Порт SMTP-сервера.
    pub port: u16,
    /// Имя пользователя SMTP.
    pub username: String,
}

/// Настройки JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtSettings {
    /// Время жизни access-токена (в минутах).
    pub access_ttl_minutes: u32,
    /// Время жизни refresh-токена (в минутах).
    pub refresh_ttl_minutes: u32,
}

/// Настройки локализации.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationSettings {
    /// Локаль по умолчанию.
    pub default_locale: String,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

#[server(GetSystemSettings, "/api")]
pub async fn fetch_system_settings() -> Result<SystemSettings, ServerFnError> {
    // TODO: Интеграция с системной конфигурацией (БД / env / файл)
    // На этапе вёрстки возвращаем мок-данные.

    Ok(SystemSettings {
        smtp: SmtpSettings {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "notifications@example.com".to_string(),
        },
        jwt: JwtSettings {
            access_ttl_minutes: 15,
            refresh_ttl_minutes: 60 * 24 * 7, // 7 дней
        },
        localization: LocalizationSettings {
            default_locale: "ru".to_string(),
        },
    })
}