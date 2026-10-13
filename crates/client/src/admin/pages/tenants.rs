// crates/client/src/admin/pages/tenants.rs
//! Управление тенантами (мультиарендность, Этап 4).
//!
//! Создание, редактирование, активация/деактивация организаций.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.

use leptos::prelude::*;
use leptos::task::spawn_local;
use rust_lms_shared::models::Tenant;

/// Страница управления тенантами (для суперадмина).
#[component]
pub fn AdminTenantList() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let tenants_resource = Resource::new(|| (), |_| fetch_all_tenants());
    let (show_create_modal, set_show_create_modal) = signal(false);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            <div class="flex items-center justify-between mb-8">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.tenants.title")
                        "Организации"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.tenants.subtitle")
                        "Управление тенантами платформы"
                    </p>
                </div>
                <button
                    on:click=move |_| set_show_create_modal.set(true)
                    class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    // i18n: t!("admin.tenants.create_button")
                    "Создать организацию"
                </button>
            </div>

            <Suspense fallback=TenantsSkeleton>
                {move || {
                    tenants_resource.get().map(|result| {
                        match result {
                            Ok(tenants) => {
                                if tenants.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <p class="text-gray-500 dark:text-gray-400">
                                                // i18n: t!("admin.tenants.empty_state")
                                                "Организации не найдены"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                                            {tenants.into_iter().map(|tenant| {
                                                view! { <TenantCard tenant=tenant /> }
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

            <Show when=move || show_create_modal.get()>
                <CreateTenantModal
                    on_close=move || set_show_create_modal.set(false)
                    on_created=move || {
                        set_show_create_modal.set(false);
                        tenants_resource.refetch();
                    }
                />
            </Show>
        </div>
    }
}

/// Карточка тенанта.
#[component]
fn TenantCard(tenant: Tenant) -> impl IntoView {
    let status_badge = if tenant.is_active {
        ("bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300", "Активна")
    } else {
        ("bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300", "Неактивна")
    };

    // ✅ Клонируем значения, используемые в view! внутри замыканий,
    // чтобы избежать E0382 (use of moved value).
    let name = tenant.name.clone();
    let slug = tenant.slug.clone();

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm hover:shadow-md transition-all duration-300 border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-start justify-between mb-4">
                <div>
                    <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-1">{name}</h3>
                    <p class="text-sm text-gray-500 dark:text-gray-400 font-mono">{slug}</p>
                </div>
                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", status_badge.0)>
                    {status_badge.1}
                </span>
            </div>

            <div class="space-y-2 text-sm text-gray-600 dark:text-gray-400 mb-6">
                <div class="flex items-center gap-2">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
                    </svg>
                    // ⚠️ В shared::Tenant нет поля `created_at` — заглушка.
                    // При подключении БД — либо добавить поле в DTO, либо
                    // тянуть из TenantRepository отдельно.
                    <span>"Дата создания не указана"</span>
                </div>
            </div>

            <div class="flex gap-2">
                <button class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors">
                    "Редактировать"
                </button>
                <button class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors">
                    "Настройки"
                </button>
            </div>
        </div>
    }
}

/// Модальное окно создания тенанта.
#[component]
fn CreateTenantModal(
    on_close: impl Fn() + Send + Sync + 'static,
    on_created: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (Fn + Send + Sync)
    let on_close = StoredValue::new(on_close);
    let on_created = StoredValue::new(on_created);

    let (name, set_name) = signal(String::new());
    let (slug, set_slug) = signal(String::new());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        // ✅ spawn_local из leptos::task
        spawn_local(async move {
            let _ = create_tenant(name.get(), slug.get()).await;
            on_created.with_value(|f| f());
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-md w-full">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-2xl font-bold text-gray-900 dark:text-white">
                        "Создать организацию"
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
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Название *"
                        </label>
                        <input
                            type="text"
                            prop:value=name
                            on:input=move |ev| set_name.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Slug (URL) *"
                        </label>
                        <input
                            type="text"
                            prop:value=slug
                            on:input=move |ev| set_slug.set(event_target_value(&ev))
                            pattern="[a-z0-9-]+"
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono"
                            required
                        />
                    </div>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700"
                        >
                            "Создать"
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

#[component]
fn TenantsSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <div class="h-48 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
            <div class="h-48 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
            <div class="h-48 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
        </div>
    }
}

// ============================================================================
// Server-функции (мок-данные для вёрстки)
// ============================================================================

#[server(GetAllTenants, "/api")]
pub async fn fetch_all_tenants() -> Result<Vec<Tenant>, ServerFnError> {
    // TODO: Интеграция с TenantRepository::find_all()
    // На этапе вёрстки возвращаем мок-данные.
    use rust_lms_shared::models::TenantId;

    let mock = vec![
        Tenant {
            id: TenantId::new(),
            slug: "acme".to_string(),
            name: "ACME Corporation".to_string(),
            is_active: true,
        },
        Tenant {
            id: TenantId::new(),
            slug: "demo".to_string(),
            name: "Demo Organization".to_string(),
            is_active: true,
        },
        Tenant {
            id: TenantId::new(),
            slug: "archived-corp".to_string(),
            name: "Archived Corp".to_string(),
            is_active: false,
        },
    ];

    Ok(mock)
}

#[server(CreateTenant, "/api")]
pub async fn create_tenant(name: String, slug: String) -> Result<Tenant, ServerFnError> {
    // TODO: Интеграция с TenantRepository::create()
    // На этапе вёрстки возвращаем мок-тенант.
    use rust_lms_shared::models::TenantId;

    let tenant = Tenant {
        id: TenantId::new(),
        slug,
        name,
        is_active: true,
    };

    Ok(tenant)
}