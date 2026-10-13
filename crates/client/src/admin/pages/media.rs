// crates/client/src/admin/pages/media.rs
//! Библиотека медиафайлов (Этап 13 Media Upload MVP).
//!
//! Управление загруженными файлами: drag & drop загрузка, превью изображений/видео,
//! фильтрация по типу, копирование URL для вставки в Markdown, статистика хранилища.
//! Интеграция с объектным хранилищем (MinIO) и метаданными в БД.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ⚠️ `MediaFile`/`MediaType` — локальные DTO: они ещё не вынесены в
//! `rust_lms_shared::models`. При интеграции с Этапом 13 — перенести в shared.
//!
//! ⚠️ `#[server]`-функция не может принимать `web_sys::File` (нет `Serialize`).
//! Загрузка эмулируется через `upload_media_file(file_name, file_size, mime_type)`.
//! Реальная загрузка будет через multipart HTTP-запрос, а не через `#[server]`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasm_bindgen::JsCast;

/// Страница управления медиафайлами.
#[component]
pub fn AdminMedia() -> impl IntoView {
    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let media_resource = Resource::new(|| (), |_| fetch_all_media());
    let stats_resource = Resource::new(|| (), |_| fetch_media_stats());

    // Сигналы состояния
    let (view_mode, set_view_mode) = signal(MediaViewMode::Grid);
    let (type_filter, set_type_filter) = signal(None::<MediaType>);
    let (search_query, set_search_query) = signal(String::new());
    let (preview_file, set_preview_file) = signal(None::<MediaFile>);
    let (delete_file, set_delete_file) = signal(None::<MediaFile>);
    let (uploading_files, set_uploading_files) = signal(Vec::<UploadingFile>::new());
    let (toast, set_toast) = signal(None::<ToastMessage>);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            // Заголовок
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-6 gap-4">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        "Медиатека"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        "Управление изображениями, видео и документами курсов"
                    </p>
                </div>

                // Статистика хранилища
                <Suspense fallback=move || view! {
                    <div class="h-16 w-64 bg-gray-200 dark:bg-gray-700 rounded-xl animate-pulse"></div>
                }>
                    {move || {
                        stats_resource.get().map(|result| {
                            match result {
                                Ok(stats) => view! {
                                    <StorageStats stats=stats />
                                }.into_any(),
                                Err(_) => view! { <div></div> }.into_any(),
                            }
                        })
                    }}
                </Suspense>
            </div>

            // Зона загрузки (Drag & Drop)
            <UploadZone
                on_files_selected=move |files| {
                    spawn_local(async move {
                        handle_upload(files, set_uploading_files, set_toast, media_resource).await;
                    });
                }
            />

            // Активные загрузки
            <Show when=move || !uploading_files.get().is_empty()>
                <div class="mb-6 space-y-2">
                    {move || {
                        uploading_files.get().into_iter().map(|file| {
                            view! { <UploadProgressItem file=file /> }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </Show>

            // Панель фильтров и переключения вида
            <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 mb-6">
                <div class="flex flex-wrap items-center gap-2">
                    <FilterChip
                        label="Все"
                        icon="📁"
                        active=move || type_filter.get().is_none()
                        on_click=move || set_type_filter.set(None)
                    />
                    <FilterChip
                        label="Изображения"
                        icon="🖼️"
                        active=move || type_filter.get() == Some(MediaType::Image)
                        on_click=move || set_type_filter.set(Some(MediaType::Image))
                    />
                    <FilterChip
                        label="Видео"
                        icon="🎬"
                        active=move || type_filter.get() == Some(MediaType::Video)
                        on_click=move || set_type_filter.set(Some(MediaType::Video))
                    />
                    <FilterChip
                        label="Документы"
                        icon="📄"
                        active=move || type_filter.get() == Some(MediaType::Document)
                        on_click=move || set_type_filter.set(Some(MediaType::Document))
                    />
                </div>

                <div class="flex items-center gap-2 w-full sm:w-auto">
                    <div class="relative flex-1 sm:flex-initial">
                        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                        </svg>
                        <input
                            type="text"
                            placeholder="Поиск по имени..."
                            prop:value=search_query
                            on:input=move |ev| set_search_query.set(event_target_value(&ev))
                            class="w-full sm:w-64 pl-9 pr-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white text-sm focus:ring-2 focus:ring-blue-500"
                        />
                    </div>

                    // Переключатель Grid / List
                    <div class="flex bg-gray-100 dark:bg-gray-800 rounded-lg p-1">
                        <button
                            on:click=move |_| set_view_mode.set(MediaViewMode::Grid)
                            class=move || {
                                format!(
                                    "p-1.5 rounded {}",
                                    if view_mode.get() == MediaViewMode::Grid {
                                        "bg-white dark:bg-gray-700 shadow-sm"
                                    } else {
                                        "text-gray-500 dark:text-gray-400"
                                    }
                                )
                            }
                            title="Сетка"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"></path>
                            </svg>
                        </button>
                        <button
                            on:click=move |_| set_view_mode.set(MediaViewMode::List)
                            class=move || {
                                format!(
                                    "p-1.5 rounded {}",
                                    if view_mode.get() == MediaViewMode::List {
                                        "bg-white dark:bg-gray-700 shadow-sm"
                                    } else {
                                        "text-gray-500 dark:text-gray-400"
                                    }
                                )
                            }
                            title="Список"
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path>
                            </svg>
                        </button>
                    </div>
                </div>
            </div>

            // Основной контент
            <Suspense fallback=MediaSkeleton>
                {move || {
                    media_resource.get().map(|result| {
                        match result {
                            Ok(files) => {
                                let filtered = files.into_iter()
                                    .filter(|f| {
                                        let matches_type = type_filter.get().is_none() ||
                                            f.media_type == type_filter.get().unwrap();
                                        let matches_search = search_query.get().is_empty() ||
                                            f.name.to_lowercase().contains(&search_query.get().to_lowercase());
                                        matches_type && matches_search
                                    })
                                    .collect::<Vec<_>>();

                                if filtered.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <div class="text-5xl mb-4">"📭"</div>
                                            <p class="text-gray-500 dark:text-gray-400 text-lg mb-1">
                                                "Файлы не найдены"
                                            </p>
                                            <p class="text-gray-400 dark:text-gray-500 text-sm">
                                                "Загрузите первый файл или измените фильтры"
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    match view_mode.get() {
                                        MediaViewMode::Grid => view! {
                                            <MediaGrid
                                                files=filtered
                                                on_preview=move |f| set_preview_file.set(Some(f))
                                                on_delete=move |f| set_delete_file.set(Some(f))
                                                on_copy_url=move |f| {
                                                    copy_to_clipboard(&f.url);
                                                    set_toast.set(Some(ToastMessage {
                                                        message: "URL скопирован в буфер обмена".to_string(),
                                                        variant: ToastVariant::Success,
                                                    }));
                                                }
                                            />
                                        }.into_any(),
                                        MediaViewMode::List => view! {
                                            <MediaList
                                                files=filtered
                                                on_preview=move |f| set_preview_file.set(Some(f))
                                                on_delete=move |f| set_delete_file.set(Some(f))
                                                on_copy_url=move |f| {
                                                    copy_to_clipboard(&f.url);
                                                    set_toast.set(Some(ToastMessage {
                                                        message: "URL скопирован".to_string(),
                                                        variant: ToastVariant::Success,
                                                    }));
                                                }
                                            />
                                        }.into_any(),
                                    }
                                }
                            }
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold mb-1">"Ошибка загрузки файлов"</p>
                                    <p class="text-sm">{e.to_string()}</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            // Модальное окно предпросмотра
            <Show when=move || preview_file.get().is_some()>
                <MediaPreviewModal
                    file=preview_file.get().unwrap()
                    on_close=move || set_preview_file.set(None)
                />
            </Show>

            // Модалка подтверждения удаления
            <Show when=move || delete_file.get().is_some()>
                <DeleteConfirmModal
                    file=delete_file.get().unwrap()
                    on_close=move || set_delete_file.set(None)
                    on_confirmed=move || {
                        let file = delete_file.get().unwrap();
                        spawn_local(async move {
                            match delete_media_file(file.id).await {
                                Ok(_) => {
                                    set_toast.set(Some(ToastMessage {
                                        message: "Файл удалён".to_string(),
                                        variant: ToastVariant::Success,
                                    }));
                                    media_resource.refetch();
                                    stats_resource.refetch();
                                }
                                Err(e) => {
                                    set_toast.set(Some(ToastMessage {
                                        message: format!("Ошибка: {}", e),
                                        variant: ToastVariant::Error,
                                    }));
                                }
                            }
                            set_delete_file.set(None);
                        });
                    }
                />
            </Show>

            // Toast-уведомления
            <Show when=move || toast.get().is_some()>
                <Toast
                    message=toast.get().unwrap()
                    on_close=move || set_toast.set(None)
                />
            </Show>
        </div>
    }
}

/// Режим отображения медиатеки.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MediaViewMode {
    /// Сетка с превью.
    Grid,
    /// Таблица со списком.
    List,
}

/// Вариант toast-сообщения.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastVariant {
    /// Успешное действие.
    Success,
    /// Ошибка.
    Error,
    /// Информация.
    Info,
}

/// Toast-сообщение.
#[derive(Clone, Debug)]
pub struct ToastMessage {
    /// Текст сообщения.
    pub message: String,
    /// Вариант (цвет).
    pub variant: ToastVariant,
}

/// Зона drag & drop для загрузки файлов.
#[component]
fn UploadZone(
    on_files_selected: impl Fn(Vec<web_sys::File>) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_files_selected = StoredValue::new(on_files_selected);

    let (is_dragging, set_is_dragging) = signal(false);
    // ✅ Leptos 0.8: NodeRef параметризуется leptos-типом элемента, не web_sys
    let file_input_ref = NodeRef::<leptos::html::Input>::new();

    let handle_drop = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        set_is_dragging.set(false);

        if let Some(data_transfer) = ev.data_transfer() {
            if let Some(files) = data_transfer.files() {
                let mut result = Vec::new();
                for i in 0..files.length() {
                    if let Some(file) = files.item(i) {
                        result.push(file);
                    }
                }
                if !result.is_empty() {
                    on_files_selected.with_value(|f| f(result));
                }
            }
        }
    };

    let handle_file_input = move |ev: web_sys::Event| {
        // ✅ JsCast в scope (импорт в начале файла)
        if let Some(input) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
            if let Some(files) = input.files() {
                let mut result = Vec::new();
                for i in 0..files.length() {
                    if let Some(file) = files.item(i) {
                        result.push(file);
                    }
                }
                if !result.is_empty() {
                    on_files_selected.with_value(|f| f(result));
                }
                // Сбрасываем input, чтобы можно было загрузить тот же файл повторно
                input.set_value("");
            }
        }
    };

    view! {
        <div
            class=move || {
                format!(
                    "mb-6 border-2 border-dashed rounded-xl p-8 text-center transition-all cursor-pointer {}",
                    if is_dragging.get() {
                        "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                    } else {
                        "border-gray-300 dark:border-gray-600 hover:border-blue-400 dark:hover:border-blue-500 bg-gray-50/50 dark:bg-gray-800/30"
                    }
                )
            }
            on:dragover=move |ev| {
                ev.prevent_default();
                ev.stop_propagation();
                set_is_dragging.set(true);
            }
            on:dragleave=move |_| set_is_dragging.set(false)
            on:drop=handle_drop
            on:click=move |_| {
                if let Some(input) = file_input_ref.get() {
                    // input — web_sys::HtmlInputElement (через ElementType::Output)
                    let _ = input.click();
                }
            }
        >
            <input
                node_ref=file_input_ref
                type="file"
                multiple
                accept="image/*,video/*,.pdf,.doc,.docx,.ppt,.pptx"
                class="hidden"
                on:change=handle_file_input
            />

            <div class="flex flex-col items-center gap-3">
                <div class=move || {
                    format!(
                        "w-16 h-16 rounded-full flex items-center justify-center text-3xl transition-all {}",
                        if is_dragging.get() {
                            "bg-blue-100 dark:bg-blue-900/30 scale-110"
                        } else {
                            "bg-gray-100 dark:bg-gray-700"
                        }
                    )
                }>
                    {if is_dragging.get() { "📥" } else { "☁️" }}
                </div>
                <div>
                    <p class="text-lg font-medium text-gray-900 dark:text-white">
                        {if is_dragging.get() {
                            "Отпустите файлы для загрузки"
                        } else {
                            "Перетащите файлы сюда"
                        }}
                    </p>
                    <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                        "или нажмите для выбора • JPG, PNG, MP4, PDF до 100MB"
                    </p>
                </div>
            </div>
        </div>
    }
}

/// Элемент прогресса загрузки файла.
#[component]
fn UploadProgressItem(file: UploadingFile) -> impl IntoView {
    let progress_color = match file.status {
        UploadStatus::Uploading => "bg-blue-500",
        UploadStatus::Completed => "bg-green-500",
        UploadStatus::Error => "bg-red-500",
    };

    let name = file.name.clone();
    let progress = file.progress;
    let icon = match file.status {
        UploadStatus::Uploading => "⏳",
        UploadStatus::Completed => "✅",
        UploadStatus::Error => "❌",
    };

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-3 flex items-center gap-3">
            <div class="text-xl">{icon}</div>
            <div class="flex-1 min-w-0">
                <div class="flex items-center justify-between mb-1">
                    <span class="text-sm font-medium text-gray-900 dark:text-white truncate">
                        {name}
                    </span>
                    <span class="text-xs text-gray-500 dark:text-gray-400 ml-2">
                        {progress} "%"
                    </span>
                </div>
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                    <div
                        class=format!("{} h-1.5 rounded-full transition-all duration-300", progress_color)
                        style:width=format!("{}%", progress)
                    ></div>
                </div>
            </div>
        </div>
    }
}

/// Карточка статистики хранилища.
#[component]
fn StorageStats(stats: MediaStats) -> impl IntoView {
    let usage_pct = if stats.total_quota_bytes > 0 {
        (stats.used_bytes as f64 / stats.total_quota_bytes as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    let bar_color = if usage_pct > 90.0 {
        "bg-red-500"
    } else if usage_pct > 70.0 {
        "bg-yellow-500"
    } else {
        "bg-blue-500"
    };

    let used_str = format_bytes(stats.used_bytes);
    let quota_str = format_bytes(stats.total_quota_bytes);
    let pct_str = format!("{:.1}%", usage_pct);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4 min-w-[260px]">
            <div class="flex items-center justify-between mb-2">
                <span class="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase">
                    "Хранилище"
                </span>
                <span class="text-xs text-gray-500 dark:text-gray-400">
                    {used_str} " / " {quota_str}
                </span>
            </div>
            <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2 mb-2">
                <div
                    class=format!("{} h-2 rounded-full transition-all", bar_color)
                    style:width=format!("{}%", usage_pct)
                ></div>
            </div>
            <div class="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
                <span>{stats.total_files} " файлов"</span>
                <span>{pct_str} " использовано"</span>
            </div>
        </div>
    }
}

/// Чип фильтра по типу медиа.
#[component]
fn FilterChip(
    label: &'static str,
    icon: &'static str,
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
                    "px-3 py-1.5 text-sm font-medium rounded-lg transition-all flex items-center gap-1.5 {}",
                    if active.with_value(|f| f()) {
                        "bg-blue-600 text-white shadow-sm"
                    } else {
                        "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-600"
                    }
                )
            }
        >
            <span>{icon}</span>
            <span>{label}</span>
        </button>
    }
}

/// Сетка медиафайлов.
#[component]
fn MediaGrid(
    files: Vec<MediaFile>,
    on_preview: impl Fn(MediaFile) + Send + Sync + 'static,
    on_delete: impl Fn(MediaFile) + Send + Sync + 'static,
    on_copy_url: impl Fn(MediaFile) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_preview = StoredValue::new(on_preview);
    let on_delete = StoredValue::new(on_delete);
    let on_copy_url = StoredValue::new(on_copy_url);

    view! {
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
            {files.into_iter().map(|file| {
                let file_for_preview = file.clone();
                let file_for_delete = file.clone();
                let file_for_copy = file.clone();
                view! {
                    <MediaGridCard
                        file=file
                        on_preview=move || on_preview.with_value(|f| f(file_for_preview.clone()))
                        on_delete=move || on_delete.with_value(|f| f(file_for_delete.clone()))
                        on_copy_url=move || on_copy_url.with_value(|f| f(file_for_copy.clone()))
                    />
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Карточка файла в сетке.
#[component]
fn MediaGridCard(
    file: MediaFile,
    on_preview: impl Fn() + Send + Sync + 'static,
    on_delete: impl Fn() + Send + Sync + 'static,
    on_copy_url: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_preview = StoredValue::new(on_preview);
    let on_delete = StoredValue::new(on_delete);
    let on_copy_url = StoredValue::new(on_copy_url);

    // ✅ Клонируем name в две переменные — Leptos view! забирает владение
    // при первом упоминании в атрибуте, поэтому для title и текста нужны разные клоны.
    let name_for_title = file.name.clone();
    let name_for_text = file.name.clone();
    let size_str = format_bytes(file.size_bytes);
    let uploaded_str = file.uploaded_at.format("%d.%m").to_string();

    view! {
        <div class="group relative bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden hover:shadow-md transition-all">
            // Превью
            <div
                class="aspect-square bg-gray-100 dark:bg-gray-900 flex items-center justify-center cursor-pointer overflow-hidden relative"
                on:click=move |_| on_preview.with_value(|f| f())
            >
                <MediaThumbnail file=file.clone() />

                // Hover overlay с действиями
                <div class="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2">
                    <button
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_copy_url.with_value(|f| f());
                        }
                        class="p-2 bg-white/90 rounded-lg hover:bg-white transition-colors"
                        title="Копировать URL"
                    >
                        <svg class="w-4 h-4 text-gray-700" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                        </svg>
                    </button>
                    <button
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_delete.with_value(|f| f());
                        }
                        class="p-2 bg-red-500/90 rounded-lg hover:bg-red-500 transition-colors"
                        title="Удалить"
                    >
                        <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                        </svg>
                    </button>
                </div>
            </div>

            // Информация
            <div class="p-2.5">
                <div class="text-xs font-medium text-gray-900 dark:text-white truncate mb-0.5" title=name_for_title>
                    {name_for_text}
                </div>
                <div class="flex items-center justify-between text-[10px] text-gray-500 dark:text-gray-400">
                    <span>{size_str}</span>
                    <span>{uploaded_str}</span>
                </div>
            </div>
        </div>
    }
}

/// Превью файла (thumbnail).
#[component]
fn MediaThumbnail(file: MediaFile) -> impl IntoView {
    match file.media_type {
        MediaType::Image => view! {
            <img
                src=file.url.clone()
                alt=file.name.clone()
                class="w-full h-full object-cover"
                loading="lazy"
            />
        }.into_any(),
        MediaType::Video => view! {
            <div class="relative w-full h-full bg-gradient-to-br from-gray-800 to-gray-900 flex items-center justify-center">
                <div class="w-14 h-14 rounded-full bg-white/20 backdrop-blur-sm flex items-center justify-center">
                    <svg class="w-7 h-7 text-white ml-1" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M8 5v14l11-7z"></path>
                    </svg>
                </div>
                <div class="absolute bottom-2 right-2 px-1.5 py-0.5 bg-black/70 text-white text-[10px] rounded">
                    "VIDEO"
                </div>
            </div>
        }.into_any(),
        MediaType::Document => {
            let icon = get_document_icon(&file.mime_type);
            let ext = get_file_extension(&file.name);
            view! {
                <div class="w-full h-full bg-gradient-to-br from-blue-50 to-blue-100 dark:from-blue-900/30 dark:to-blue-800/30 flex flex-col items-center justify-center">
                    <div class="text-4xl mb-1">{icon}</div>
                    <div class="text-[10px] font-mono text-blue-700 dark:text-blue-300 uppercase">
                        {ext}
                    </div>
                </div>
            }.into_any()
        },
        MediaType::Other => view! {
            <div class="w-full h-full bg-gradient-to-br from-gray-100 to-gray-200 dark:from-gray-800 dark:to-gray-700 flex items-center justify-center">
                <div class="text-4xl">"📦"</div>
            </div>
        }.into_any(),
    }
}

/// Список медиафайлов (табличный вид).
#[component]
fn MediaList(
    files: Vec<MediaFile>,
    on_preview: impl Fn(MediaFile) + Send + Sync + 'static,
    on_delete: impl Fn(MediaFile) + Send + Sync + 'static,
    on_copy_url: impl Fn(MediaFile) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_preview = StoredValue::new(on_preview);
    let on_delete = StoredValue::new(on_delete);
    let on_copy_url = StoredValue::new(on_copy_url);

    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
            <div class="overflow-x-auto">
                <table class="w-full">
                    <thead class="bg-gray-50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700">
                        <tr>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Файл"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Тип"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Размер"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Загружен"</th>
                            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">"Действия"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                        {files.into_iter().map(|file| {
                            let file_for_preview = file.clone();
                            let file_for_delete = file.clone();
                            let file_for_copy = file.clone();
                            view! {
                                <MediaListRow
                                    file=file
                                    on_preview=move || on_preview.with_value(|f| f(file_for_preview.clone()))
                                    on_delete=move || on_delete.with_value(|f| f(file_for_delete.clone()))
                                    on_copy_url=move || on_copy_url.with_value(|f| f(file_for_copy.clone()))
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// Строка файла в списке.
#[component]
fn MediaListRow(
    file: MediaFile,
    on_preview: impl Fn() + Send + Sync + 'static,
    on_delete: impl Fn() + Send + Sync + 'static,
    on_copy_url: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_preview = StoredValue::new(on_preview);
    let on_delete = StoredValue::new(on_delete);
    let on_copy_url = StoredValue::new(on_copy_url);

    let type_badge = match file.media_type {
        MediaType::Image => ("bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300", "🖼️ Изображение"),
        MediaType::Video => ("bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300", "🎬 Видео"),
        MediaType::Document => ("bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300", "📄 Документ"),
        MediaType::Other => ("bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200", "📦 Другое"),
    };

    let name = file.name.clone();
    let url = file.url.clone();
    let size_str = format_bytes(file.size_bytes);
    let uploaded_str = file.uploaded_at.format("%d.%m.%Y %H:%M").to_string();

    view! {
        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
            <td class="px-4 py-3">
                <button
                    on:click=move |_| on_preview.with_value(|f| f())
                    class="flex items-center gap-3 text-left hover:opacity-80 transition-opacity"
                >
                    <div class="w-10 h-10 rounded-lg bg-gray-100 dark:bg-gray-700 flex items-center justify-center text-xl flex-shrink-0 overflow-hidden">
                        <MediaThumbnail file=file.clone() />
                    </div>
                    <div class="min-w-0">
                        <div class="text-sm font-medium text-gray-900 dark:text-white truncate max-w-xs">
                            {name.clone()}
                        </div>
                        <div class="text-xs text-gray-500 dark:text-gray-400 font-mono truncate max-w-xs">
                            {url.clone()}
                        </div>
                    </div>
                </button>
            </td>
            <td class="px-4 py-3">
                <span class=format!("inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium {}", type_badge.0)>
                    {type_badge.1}
                </span>
            </td>
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                {size_str}
            </td>
            <td class="px-4 py-3 text-sm text-gray-600 dark:text-gray-400">
                {uploaded_str}
            </td>
            <td class="px-4 py-3 text-right">
                <div class="flex items-center justify-end gap-1">
                    <button
                        on:click=move |_| on_copy_url.with_value(|f| f())
                        class="p-1.5 text-gray-500 hover:text-blue-600 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded transition-colors"
                        title="Копировать URL"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                        </svg>
                    </button>
                    <button
                        on:click=move |_| on_preview.with_value(|f| f())
                        class="p-1.5 text-gray-500 hover:text-blue-600 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded transition-colors"
                        title="Предпросмотр"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                        </svg>
                    </button>
                    <button
                        on:click=move |_| on_delete.with_value(|f| f())
                        class="p-1.5 text-gray-500 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
                        title="Удалить"
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

/// Модальное окно предпросмотра файла.
#[component]
fn MediaPreviewModal(
    file: MediaFile,
    on_close: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_close = StoredValue::new(on_close);

    let name = file.name.clone();
    let url = file.url.clone();
    let mime = file.mime_type.clone();
    let size_str = format_bytes(file.size_bytes);
    let uploaded_str = file.uploaded_at.format("%d.%m.%Y %H:%M").to_string();
    let markdown = format!("![alt]({})", url);
    let media_type = file.media_type;
    let url_for_img = file.url.clone();
    let name_for_alt = file.name.clone();
    let url_for_video = file.url.clone();
    let mime_for_video = file.mime_type.clone();
    let url_for_download = file.url.clone();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col">
                // Заголовок
                <div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
                    <div class="min-w-0 flex-1 mr-4">
                        <h3 class="text-lg font-semibold text-gray-900 dark:text-white truncate">
                            {name.clone()}
                        </h3>
                        <p class="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">
                            {url.clone()}
                        </p>
                    </div>
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                // Контент
                <div class="flex-1 overflow-auto p-6 bg-gray-50 dark:bg-gray-900/50 flex items-center justify-center">
                    {match media_type {
                        MediaType::Image => view! {
                            <img src=url_for_img alt=name_for_alt class="max-w-full max-h-[60vh] object-contain rounded-lg shadow-lg" />
                        }.into_any(),
                        MediaType::Video => view! {
                            <video controls class="max-w-full max-h-[60vh] rounded-lg shadow-lg">
                                <source src=url_for_video type=mime_for_video />
                                "Ваш браузер не поддерживает видео"
                            </video>
                        }.into_any(),
                        MediaType::Document => {
                            let doc_icon = get_document_icon(&mime);
                            let url_for_download_2 = url_for_download.clone();
                            view! {
                                <div class="text-center">
                                    <div class="text-8xl mb-4">{doc_icon}</div>
                                    <p class="text-gray-600 dark:text-gray-400 mb-4">"Предпросмотр документов недоступен"</p>
                                    <a
                                        href=url_for_download_2
                                        target="_blank"
                                        class="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                                    >
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"></path>
                                        </svg>
                                        "Скачать файл"
                                    </a>
                                </div>
                            }.into_any()
                        },
                        MediaType::Other => view! {
                            <div class="text-center">
                                <div class="text-8xl mb-4">"📦"</div>
                                <p class="text-gray-600 dark:text-gray-400">"Предпросмотр недоступен"</p>
                            </div>
                        }.into_any(),
                    }}
                </div>

                // Информация о файле
                <div class="p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                        <div>
                            <div class="text-xs text-gray-500 dark:text-gray-400 mb-0.5">"Размер"</div>
                            <div class="font-medium text-gray-900 dark:text-white">{size_str}</div>
                        </div>
                        <div>
                            <div class="text-xs text-gray-500 dark:text-gray-400 mb-0.5">"MIME-тип"</div>
                            <div class="font-mono text-gray-900 dark:text-white text-xs">{mime}</div>
                        </div>
                        <div>
                            <div class="text-xs text-gray-500 dark:text-gray-400 mb-0.5">"Загружен"</div>
                            <div class="font-medium text-gray-900 dark:text-white">{uploaded_str}</div>
                        </div>
                        <div>
                            <div class="text-xs text-gray-500 dark:text-gray-400 mb-0.5">"Markdown"</div>
                            <code class="text-xs font-mono text-blue-600 dark:text-blue-400 truncate block">
                                {markdown}
                            </code>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Модалка подтверждения удаления.
#[component]
fn DeleteConfirmModal(
    file: MediaFile,
    on_close: impl Fn() + Send + Sync + 'static,
    on_confirmed: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks
    let on_close = StoredValue::new(on_close);
    let on_confirmed = StoredValue::new(on_confirmed);

    let name = file.name.clone();
    let size_str = format_bytes(file.size_bytes);
    let uploaded_str = file.uploaded_at.format("%d.%m.%Y").to_string();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-md w-full">
                <div class="p-6">
                    <div class="flex items-center gap-3 mb-4">
                        <div class="w-12 h-12 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center flex-shrink-0">
                            <svg class="w-6 h-6 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                            </svg>
                        </div>
                        <div>
                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">"Удалить файл?"</h3>
                            <p class="text-sm text-gray-600 dark:text-gray-400">"Это действие нельзя отменить"</p>
                        </div>
                    </div>

                    <div class="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg mb-4">
                        <div class="text-sm font-medium text-gray-900 dark:text-white truncate">{name}</div>
                        <div class="text-xs text-gray-500 dark:text-gray-400">{size_str} " • " {uploaded_str}</div>
                    </div>

                    <div class="p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg text-sm text-yellow-800 dark:text-yellow-300 mb-4">
                        "⚠️ Если файл используется в уроках, ссылки на него перестанут работать."
                    </div>

                    <div class="flex gap-3">
                        <button
                            on:click=move |_| on_confirmed.with_value(|f| f())
                            class="flex-1 px-4 py-2 bg-red-600 text-white font-medium rounded-lg hover:bg-red-700 transition-colors"
                        >
                            "Удалить"
                        </button>
                        <button
                            on:click=move |_| on_close.with_value(|f| f())
                            class="flex-1 px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                        >
                            "Отмена"
                        </button>
                    </div>
                </div>
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

/// Скелетон загрузки.
#[component]
fn MediaSkeleton() -> impl IntoView {
    view! {
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4 animate-pulse">
            {(0..10).map(|_| {
                view! {
                    <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
                        <div class="aspect-square bg-gray-200 dark:bg-gray-700"></div>
                        <div class="p-2.5 space-y-1">
                            <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
                            <div class="h-2 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

// Вспомогательные функции

/// Форматирование размера файла в человекочитаемый вид.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} ГБ", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} МБ", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} КБ", bytes as f64 / KB as f64)
    } else {
        format!("{} Б", bytes)
    }
}

/// Получение иконки для документа по MIME-типу.
fn get_document_icon(mime_type: &str) -> &'static str {
    match mime_type {
        "application/pdf" => "📕",
        t if t.contains("word") || t.contains("document") => "📘",
        t if t.contains("sheet") || t.contains("excel") => "📗",
        t if t.contains("presentation") || t.contains("powerpoint") => "📙",
        _ => "📄",
    }
}

/// Получение расширения файла из имени.
fn get_file_extension(name: &str) -> String {
    name.rsplit('.').next().unwrap_or("").to_uppercase()
}

/// Копирование текста в буфер обмена.
fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.navigator().clipboard().write_text(text);
    }
}

/// Обработка загрузки файлов.
///
/// ⚠️ На этапе вёрстки эмулирует загрузку: для каждого файла извлекаем
/// name/size/mime и вызываем server-функцию (пока мок). Реальная загрузка
/// будет через multipart HTTP-запрос в отдельный handler.
async fn handle_upload(
    files: Vec<web_sys::File>,
    set_uploading_files: WriteSignal<Vec<UploadingFile>>,
    set_toast: WriteSignal<Option<ToastMessage>>,
    media_resource: Resource<Result<Vec<MediaFile>, ServerFnError>>,
) {
    let mut uploading = Vec::new();
    for file in &files {
        uploading.push(UploadingFile {
            name: file.name(),
            progress: 0,
            status: UploadStatus::Uploading,
        });
    }
    set_uploading_files.set(uploading);

    let total = files.len();
    let mut success_count = 0;

    for (idx, file) in files.into_iter().enumerate() {
        // ✅ Извлекаем Serializable-поля, т.к. web_sys::File нельзя передать в #[server]
        let file_name = file.name();
        let file_size = file.size() as u64;
        let mime_type = file.type_();

        match upload_media_file(file_name, file_size, mime_type).await {
            Ok(_) => {
                success_count += 1;
                set_uploading_files.update(|v| {
                    if let Some(f) = v.get_mut(idx) {
                        f.progress = 100;
                        f.status = UploadStatus::Completed;
                    }
                });
            }
            Err(_) => {
                set_uploading_files.update(|v| {
                    if let Some(f) = v.get_mut(idx) {
                        f.status = UploadStatus::Error;
                    }
                });
            }
        }
    }

    if success_count == total {
        set_toast.set(Some(ToastMessage {
            message: format!("Загружено файлов: {}", success_count),
            variant: ToastVariant::Success,
        }));
    } else if success_count > 0 {
        set_toast.set(Some(ToastMessage {
            message: format!("Загружено {}/{}. Часть файлов с ошибками.", success_count, total),
            variant: ToastVariant::Info,
        }));
    } else {
        set_toast.set(Some(ToastMessage {
            message: "Ошибка загрузки файлов".to_string(),
            variant: ToastVariant::Error,
        }));
    }

    // Очищаем список загрузок сразу после refetch
    media_resource.refetch();
    set_uploading_files.set(Vec::new());
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// Тип медиафайла.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    /// Изображение.
    Image,
    /// Видео.
    Video,
    /// Документ.
    Document,
    /// Другое.
    Other,
}

/// Медиафайл.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    /// ID файла.
    pub id: Uuid,
    /// Имя файла.
    pub name: String,
    /// URL (публичный или signed).
    pub url: String,
    /// MIME-тип.
    pub mime_type: String,
    /// Размер в байтах.
    pub size_bytes: u64,
    /// Тип медиа.
    pub media_type: MediaType,
    /// Когда загружен.
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
}

/// Статус загрузки файла.
#[derive(Clone, Debug, PartialEq)]
pub enum UploadStatus {
    /// Идёт загрузка.
    Uploading,
    /// Загрузка завершена.
    Completed,
    /// Ошибка загрузки.
    Error,
}

/// Файл в процессе загрузки.
#[derive(Clone, Debug)]
pub struct UploadingFile {
    /// Имя файла.
    pub name: String,
    /// Прогресс (0-100).
    pub progress: u8,
    /// Статус.
    pub status: UploadStatus,
}

/// Статистика хранилища.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaStats {
    /// Общее количество файлов.
    pub total_files: usize,
    /// Использовано байт.
    pub used_bytes: u64,
    /// Квота хранилища (байт).
    pub total_quota_bytes: u64,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

/// Получение списка всех медиафайлов тенанта.
#[server(GetAllMedia, "/api")]
pub async fn fetch_all_media() -> Result<Vec<MediaFile>, ServerFnError> {
    // TODO: Интеграция с MediaRepository::list_by_tenant() из Этапа 13
    // На этапе вёрстки возвращаем 6 мок-файлов.
    use chrono::{Duration, Utc};

    let now = Utc::now();

    let mock = vec![
        MediaFile {
            id: Uuid::new_v4(),
            name: "rust-logo.png".to_string(),
            url: "https://cdn.example.com/media/rust-logo.png".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: 45_000,
            media_type: MediaType::Image,
            uploaded_at: now - Duration::days(5),
        },
        MediaFile {
            id: Uuid::new_v4(),
            name: "lesson-01-intro.mp4".to_string(),
            url: "https://cdn.example.com/media/lesson-01-intro.mp4".to_string(),
            mime_type: "video/mp4".to_string(),
            size_bytes: 85_000_000,
            media_type: MediaType::Video,
            uploaded_at: now - Duration::days(3),
        },
        MediaFile {
            id: Uuid::new_v4(),
            name: "cargo-cheatsheet.pdf".to_string(),
            url: "https://cdn.example.com/media/cargo-cheatsheet.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 1_250_000,
            media_type: MediaType::Document,
            uploaded_at: now - Duration::days(10),
        },
        MediaFile {
            id: Uuid::new_v4(),
            name: "diagram-ownership.png".to_string(),
            url: "https://cdn.example.com/media/diagram-ownership.png".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: 220_000,
            media_type: MediaType::Image,
            uploaded_at: now - Duration::days(2),
        },
        MediaFile {
            id: Uuid::new_v4(),
            name: "lecture-notes.docx".to_string(),
            url: "https://cdn.example.com/media/lecture-notes.docx".to_string(),
            mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
            size_bytes: 75_000,
            media_type: MediaType::Document,
            uploaded_at: now - Duration::days(7),
        },
        MediaFile {
            id: Uuid::new_v4(),
            name: "demo-archive.zip".to_string(),
            url: "https://cdn.example.com/media/demo-archive.zip".to_string(),
            mime_type: "application/zip".to_string(),
            size_bytes: 12_000_000,
            media_type: MediaType::Other,
            uploaded_at: now - Duration::days(15),
        },
    ];

    Ok(mock)
}

/// Получение статистики хранилища.
#[server(GetMediaStats, "/api")]
pub async fn fetch_media_stats() -> Result<MediaStats, ServerFnError> {
    // TODO: SQL: SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM media WHERE tenant_id = $1
    // На этапе вёрстки возвращаем мок-статистику (≈98 МБ использовано из 1 ГБ).
    Ok(MediaStats {
        total_files: 6,
        used_bytes: 98_590_000,
        total_quota_bytes: 1_073_741_824,
    })
}

/// Загрузка файла.
///
/// ⚠️ `#[server]`-функция не может принимать `web_sys::File` (нет `Serialize`).
/// Принимаем только сериализуемые поля. Реальная загрузка будет через
/// multipart HTTP-запрос в отдельный handler (например, `/api/v1/media/upload`).
#[server(UploadMediaFile, "/api")]
pub async fn upload_media_file(
    file_name: String,
    file_size: u64,
    mime_type: String,
) -> Result<MediaFile, ServerFnError> {
    // TODO: Реальная загрузка в MinIO + INSERT в таблицу media
    // На этапе вёрстки возвращаем мок-файл на основе переданных полей.

    let media_type = if mime_type.starts_with("image/") {
        MediaType::Image
    } else if mime_type.starts_with("video/") {
        MediaType::Video
    } else if mime_type == "application/pdf"
        || mime_type.contains("word")
        || mime_type.contains("document")
        || mime_type.contains("sheet")
        || mime_type.contains("presentation")
    {
        MediaType::Document
    } else {
        MediaType::Other
    };

    let file = MediaFile {
        id: Uuid::new_v4(),
        name: file_name.clone(),
        url: format!("https://cdn.example.com/media/{}", file_name),
        mime_type,
        size_bytes: file_size,
        media_type,
        uploaded_at: chrono::Utc::now(),
    };

    Ok(file)
}

/// Удаление файла.
#[server(DeleteMediaFile, "/api")]
pub async fn delete_media_file(_file_id: Uuid) -> Result<(), ServerFnError> {
    // TODO: DELETE /api/v1/media/:id + удаление из объектного хранилища
    Ok(())
}