// crates/client/src/student/pages/profile.rs
//! Страница профиля пользователя с управлением настройками и сертификатами.
//!
//! Отображает данные Identity (email), позволяет сменить пароль,
//! управлять настройками уведомлений и просматривать сертификаты (Этап 11).
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ `Certificate` из `shared` НЕ содержит `course_title`. Название целевой сущности
//! подтягивается отдельно (см. `CertificateService::get_certificate_with_details`),
//! а на странице профиля показываем `target_type` + короткий `target_id`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use rust_lms_shared::models::{Certificate, Identity, User};

/// Компонент страницы профиля.
#[component]
pub fn ProfilePage() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let profile_resource = Resource::new(|| (), |_| fetch_user_profile());
    let certificates_resource = Resource::new(|| (), |_| fetch_user_certificates());

    // Сигналы для форм
    let (show_password_form, set_show_password_form) = signal(false);
    let (show_notifications_form, set_show_notifications_form) = signal(false);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-4xl">
            <h1 class="text-3xl font-bold mb-8 text-gray-900 dark:text-white">
                // i18n: t!("student.profile.title")
                "Мой профиль"
            </h1>

            <Suspense fallback=ProfileSkeleton>
                {move || {
                    profile_resource.get().map(|result| {
                        match result {
                            Ok(profile) => view! {
                                <div class="space-y-6">
                                    // Секция основной информации
                                    <ProfileInfoSection profile=profile.clone() />

                                    // Секция смены пароля
                                    <PasswordSection
                                        // ✅ ReadSignal<bool> → Signal<bool>
                                        show_form=show_password_form.into()
                                        on_toggle=move || set_show_password_form.update(|v| *v = !*v)
                                    />

                                    // Секция настроек уведомлений
                                    <NotificationsSection
                                        // ✅ ReadSignal<bool> → Signal<bool>
                                        show_form=show_notifications_form.into()
                                        on_toggle=move || set_show_notifications_form.update(|v| *v = !*v)
                                    />

                                    // Секция сертификатов (Этап 11)
                                    <CertificatesSection certificates_resource=certificates_resource />
                                </div>
                            }.into_any(),
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold">"Ошибка загрузки профиля"</p>
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

/// Секция с основной информацией о пользователе.
#[component]
fn ProfileInfoSection(profile: UserProfile) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">
                // i18n: t!("student.profile.info_section_title")
                "Основная информация"
            </h2>

            <div class="space-y-4">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        // i18n: t!("student.profile.email_label")
                        "Email"
                    </label>
                    <p class="text-gray-900 dark:text-white font-medium">
                        {profile.identity.email}
                    </p>
                </div>

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        // i18n: t!("student.profile.full_name_label")
                        "Полное имя"
                    </label>
                    <p class="text-gray-900 dark:text-white">
                        // ⚠️ full_name отсутствует в Identity (по текущей модели).
                        // При подключении БД — либо добавить поле в Identity, либо тянуть из credentials.
                        "Не указано"
                    </p>
                </div>

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        // i18n: t!("student.profile.tenant_label")
                        "Организация"
                    </label>
                    <p class="text-gray-900 dark:text-white">
                        {profile.tenant_name}
                    </p>
                </div>

                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        // i18n: t!("student.profile.member_since_label")
                        "Участник с"
                    </label>
                    <p class="text-gray-900 dark:text-white">
                        // ⚠️ created_at отсутствует в User; заглушка до подключения БД.
                        "01.01.2026"
                    </p>
                </div>
            </div>
        </div>
    }
}

/// Секция смены пароля.
#[component]
fn PasswordSection(
    show_form: Signal<bool>,
    on_toggle: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback — чтобы замыкание было Fn (не FnOnce) и Send + Sync
    let on_toggle = StoredValue::new(on_toggle);

    let (current_password, set_current_password) = signal(String::new());
    let (new_password, set_new_password) = signal(String::new());
    let (confirm_password, set_confirm_password) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (success, set_success) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        if new_password.get() != confirm_password.get() {
            set_error.set(Some("Пароли не совпадают".to_string()));
            return;
        }

        // ✅ spawn_local из leptos::task
        spawn_local(async move {
            match change_password(
                current_password.get(),
                new_password.get(),
            ).await {
                Ok(_) => {
                    set_success.set(true);
                    set_current_password.set(String::new());
                    set_new_password.set(String::new());
                    set_confirm_password.set(String::new());
                }
                Err(e) => set_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
                    // i18n: t!("student.profile.password_section_title")
                    "Безопасность"
                </h2>
                <button
                    on:click=move |_| on_toggle.with_value(|f| f())
                    class="px-4 py-2 text-sm font-medium text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 transition-colors"
                >
                    // i18n: t!("student.profile.change_password_button")
                    "Сменить пароль"
                </button>
            </div>

            <Show when=move || show_form.get()>
                <form on:submit=on_submit class="space-y-4 mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
                    <Show when=move || error.get().is_some()>
                        <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg text-sm">
                            {error.get().unwrap()}
                        </div>
                    </Show>

                    <Show when=move || success.get()>
                        <div class="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 text-green-700 dark:text-green-300 rounded-lg text-sm">
                            // i18n: t!("student.profile.password_changed_success")
                            "Пароль успешно изменён"
                        </div>
                    </Show>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            // i18n: t!("student.profile.current_password_label")
                            "Текущий пароль"
                        </label>
                        <input
                            type="password"
                            prop:value=current_password
                            on:input=move |ev| set_current_password.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            // i18n: t!("student.profile.new_password_label")
                            "Новый пароль"
                        </label>
                        <input
                            type="password"
                            prop:value=new_password
                            on:input=move |ev| set_new_password.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            required
                            minlength="8"
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            // i18n: t!("student.profile.confirm_password_label")
                            "Подтвердите новый пароль"
                        </label>
                        <input
                            type="password"
                            prop:value=confirm_password
                            on:input=move |ev| set_confirm_password.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            required
                        />
                    </div>

                    <div class="flex gap-3">
                        <button
                            type="submit"
                            class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                        >
                            // i18n: t!("student.profile.save_password_button")
                            "Сохранить"
                        </button>
                        <button
                            type="button"
                            on:click=move |_| on_toggle.with_value(|f| f())
                            class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                        >
                            // i18n: t!("student.profile.cancel_button")
                            "Отмена"
                        </button>
                    </div>
                </form>
            </Show>
        </div>
    }
}

/// Секция настроек уведомлений.
#[component]
fn NotificationsSection(
    show_form: Signal<bool>,
    on_toggle: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_toggle = StoredValue::new(on_toggle);

    let (email_notifications, set_email_notifications) = signal(true);
    let (course_updates, set_course_updates) = signal(true);
    let (certificate_notifications, set_certificate_notifications) = signal(true);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
                    // i18n: t!("student.profile.notifications_section_title")
                    "Уведомления"
                </h2>
                <button
                    on:click=move |_| on_toggle.with_value(|f| f())
                    class="px-4 py-2 text-sm font-medium text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 transition-colors"
                >
                    // i18n: t!("student.profile.edit_notifications_button")
                    "Настроить"
                </button>
            </div>

            <Show when=move || show_form.get()>
                <div class="space-y-4 mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
                    <label class="flex items-center gap-3 cursor-pointer">
                        <input
                            type="checkbox"
                            prop:checked=email_notifications
                            on:change=move |ev| set_email_notifications.set(event_target_checked(&ev))
                            class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                        />
                        <span class="text-gray-700 dark:text-gray-300">
                            // i18n: t!("student.profile.email_notifications_label")
                            "Получать уведомления по email"
                        </span>
                    </label>

                    <label class="flex items-center gap-3 cursor-pointer">
                        <input
                            type="checkbox"
                            prop:checked=course_updates
                            on:change=move |ev| set_course_updates.set(event_target_checked(&ev))
                            class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                        />
                        <span class="text-gray-700 dark:text-gray-300">
                            // i18n: t!("student.profile.course_updates_label")
                            "Уведомления о новых уроках и обновлениях курсов"
                        </span>
                    </label>

                    <label class="flex items-center gap-3 cursor-pointer">
                        <input
                            type="checkbox"
                            prop:checked=certificate_notifications
                            on:change=move |ev| set_certificate_notifications.set(event_target_checked(&ev))
                            class="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                        />
                        <span class="text-gray-700 dark:text-gray-300">
                            // i18n: t!("student.profile.certificate_notifications_label")
                            "Уведомления о выдаче сертификатов"
                        </span>
                    </label>

                    <button
                        class="px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                    >
                        // i18n: t!("student.profile.save_notifications_button")
                        "Сохранить настройки"
                    </button>
                </div>
            </Show>
        </div>
    }
}

/// Секция сертификатов (Этап 11).
#[component]
fn CertificatesSection(
    // ✅ Leptos 0.8: Resource<T> с одним параметром (T = Result<Output, Error>)
    certificates_resource: Resource<Result<Vec<Certificate>, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
            <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">
                // i18n: t!("student.profile.certificates_section_title")
                "Мои сертификаты"
            </h2>

            <Suspense fallback=CertificatesSkeleton>
                {move || {
                    certificates_resource.get().map(|result| {
                        match result {
                            Ok(certificates) => {
                                if certificates.is_empty() {
                                    view! {
                                        <div class="text-center py-8">
                                            <p class="text-gray-500 dark:text-gray-400">
                                                // i18n: t!("student.profile.no_certificates")
                                                "У вас пока нет сертификатов"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="space-y-3">
                                            {certificates.into_iter().map(|cert| {
                                                view! { <CertificateCard certificate=cert /> }
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
        </div>
    }
}

/// Карточка отдельного сертификата.
///
/// ⚠️ `Certificate` не содержит `course_title`. Показываем `target_type`
/// и короткий `target_id`. Название курса/программы/потока можно получить
/// отдельным запросом на сервере (см. `CertificateService::get_certificate_with_details`).
#[component]
fn CertificateCard(certificate: Certificate) -> impl IntoView {
    let issued_date = certificate.issued_at.format("%d.%m.%Y").to_string();

    // Человекочитаемая метка типа цели
    let target_label = match certificate.target_type.as_str() {
        "course" => "Курс",
        "program" => "Программа",
        "batch" => "Поток",
        _ => "Сертификат",
    };

    // Короткий идентификатор (первые 8 символов UUID) — пока не подключена БД
    let short_target_id = certificate.target_id.to_string()[..8].to_string();
    let display_title = format!("{} {}", target_label, short_target_id);

    // ✅ Клонируем значения, которые используются в view! более одного раза,
    // чтобы избежать ошибки E0382 (borrow of moved value).
    let verification_hash_for_text = certificate.verification_hash.clone();
    let verification_hash_for_href = certificate.verification_hash.clone();
    let certificate_id = certificate.id;

    view! {
        <div class="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg border border-gray-200 dark:border-gray-600">
            <div class="flex-1">
                <h3 class="font-semibold text-gray-900 dark:text-white mb-1">
                    {display_title}
                </h3>
                <p class="text-sm text-gray-600 dark:text-gray-400">
                    // i18n: t!("student.profile.certificate_issued_date")
                    "Выдан: " {issued_date}
                </p>
                <p class="text-xs text-gray-500 dark:text-gray-500 mt-1 font-mono">
                    // i18n: t!("student.profile.certificate_verification_code")
                    "Код верификации: " {verification_hash_for_text}
                </p>
            </div>

            <div class="flex gap-2">
                <a
                    href=format!("/certificates/verify/{}", verification_hash_for_href)
                    target="_blank"
                    class="px-3 py-1.5 text-sm font-medium text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 transition-colors"
                >
                    // i18n: t!("student.profile.certificate_verify_link")
                    "Проверить"
                </a>
                <a
                    href=format!("/api/v1/certificates/{}/download", certificate_id)
                    class="px-3 py-1.5 text-sm font-medium bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                >
                    // i18n: t!("student.profile.certificate_download_button")
                    "Скачать PDF"
                </a>
            </div>
        </div>
    }
}

// Скелетоны загрузки

#[component]
fn ProfileSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-6 animate-pulse">
            <div class="h-64 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-48 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
            <div class="h-48 bg-gray-200 dark:bg-gray-700 rounded-xl"></div>
        </div>
    }
}

#[component]
fn CertificatesSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-3 animate-pulse">
            <div class="h-20 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
            <div class="h-20 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// DTO профиля пользователя.
///
/// Локальный DTO: объединяет Identity (email), User и название тенанта.
/// ⚠️ При подключении БД — заполнять из соответствующих репозиториев.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Данные личности (email и т.д.).
    pub identity: Identity,
    /// Данные пользователя в тенанте.
    pub user: User,
    /// Название организации (тенанта).
    pub tenant_name: String,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

#[server(GetUserProfile, "/api")]
pub async fn fetch_user_profile() -> Result<UserProfile, ServerFnError> {
    // ✅ Локальные импорты — только для server-части
    use rust_lms_shared::models::{IdentityId, TenantId, UserId};

    // TODO: Интеграция с IdentityRepository и UserRepository

    // Мок-данные: все поля заполнены согласно актуальным моделям.
    let identity_id = IdentityId::new();
    let tenant_id = TenantId::new();

    Ok(UserProfile {
        identity: Identity {
            id: identity_id,
            email: "student@example.com".to_string(),
            preferred_tenant_id: Some(tenant_id),
        },
        user: User {
            id: UserId::new(),
            tenant_id,
            identity_id,
            is_active: true,
        },
        tenant_name: "Demo Organization".to_string(),
    })
}

#[server(GetUserCertificates, "/api")]
pub async fn fetch_user_certificates() -> Result<Vec<Certificate>, ServerFnError> {
    // TODO: Интеграция с CertificateRepository::list_by_user() из Этапа 11
    // На этапе вёрстки возвращаем один мок-сертификат для визуальной проверки.
    use chrono::Utc;
    use uuid::Uuid;

    let mock = Certificate {
        id: Uuid::new_v4(),
        tenant_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        target_type: "course".to_string(),
        target_id: Uuid::new_v4(),
        verification_hash: "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456".to_string(),
        issued_at: Utc::now(),
    };

    Ok(vec![mock])
}

#[server(ChangePassword, "/api")]
pub async fn change_password(current: String, new: String) -> Result<(), ServerFnError> {
    // TODO: Интеграция с PasswordHasher и IdentityRepository
    // На этапе вёрстки — заглушка, всегда Ok.
    let _ = (current, new);
    Ok(())
}