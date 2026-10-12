// crates/client/src/admin/pages/lessons.rs
//! Управление уроками: создание, редактирование контента, структура курса.
//!
//! Интеграция с Этапом 5 (Content Hierarchy) — работа с ltree.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.
//!
//! ## Подход к рендеру дерева
//!
//! `fetch_course_structure` возвращает `Vec<Node>` — плоский список всех узлов курса
//! (это то, что отдаёт `NodeRepository::find_course_tree()` через ltree).
//! На клиенте мы:
//! 1) строим `HashMap<Option<Uuid>, Vec<Node>>` (Adjacency List),
//! 2) «сплющиваем» дерево в `Vec<(Node, usize)>`, где `usize` — глубина,
//! 3) рендерим линейно с отступом `ml-{depth * 6}`.
//!
//! Это даёт ту же вложенную визуализацию, что и рекурсивный рендер,
//! но без lifetime-проблем Leptos `view!` (рекурсивная функция с `&HashMap`
//! не может вернуть `impl IntoView` с `'static`).

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;
use uuid::Uuid;
use rust_lms_shared::models::{Node, NodeType};

/// Страница структуры курса с управлением главами и уроками.
#[component]
pub fn CourseStructurePage() -> impl IntoView {
    let params = use_params_map();
    // ✅ Leptos 0.8: ParamsMap::get() возвращает Option<String> (владеющий),
    // .cloned() не нужен.
    let course_id = move || params.with(|p| p.get("course_id").unwrap_or_default());

    // ✅ Leptos 0.8: Resource::new вместо create_resource
    let structure_resource = Resource::new(course_id, |id| fetch_course_structure(id));
    let (show_add_node_modal, set_show_add_node_modal) = signal(false);
    let (selected_parent, set_selected_parent) = signal(None::<Uuid>);

    view! {
        <div class="container mx-auto px-4 py-8 max-w-5xl">
            <div class="flex items-center justify-between mb-8">
                <div>
                    <h1 class="text-3xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("admin.structure.title")
                        "Структура курса"
                    </h1>
                    <p class="text-gray-600 dark:text-gray-400 mt-1">
                        // i18n: t!("admin.structure.subtitle")
                        "Управляйте главами и уроками курса"
                    </p>
                </div>
                <button
                    on:click=move |_| {
                        set_selected_parent.set(None);
                        set_show_add_node_modal.set(true);
                    }
                    class="px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors shadow-sm flex items-center gap-2"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                    </svg>
                    // i18n: t!("admin.structure.add_button")
                    "Добавить элемент"
                </button>
            </div>

            <Suspense fallback=StructureSkeleton>
                {move || {
                    structure_resource.get().map(|result| {
                        match result {
                            Ok(nodes) => {
                                if nodes.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            <p class="text-gray-500 dark:text-gray-400 mb-4">
                                                // i18n: t!("admin.structure.empty_state")
                                                "Курс пока пуст. Добавьте первую главу или урок."
                                            </p>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                                            <NodeTree
                                                nodes=nodes
                                                on_add_child=move |parent_id| {
                                                    set_selected_parent.set(Some(parent_id));
                                                    set_show_add_node_modal.set(true);
                                                }
                                            />
                                        </div>
                                    }.into_any()
                                }
                            }
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    <p class="font-semibold">"Ошибка загрузки структуры"</p>
                                    <p class="text-sm">{e.to_string()}</p>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>

            <Show when=move || show_add_node_modal.get()>
                <AddNodeModal
                    course_id=course_id()
                    parent_id=selected_parent.get()
                    on_close=move || set_show_add_node_modal.set(false)
                    on_created=move || {
                        set_show_add_node_modal.set(false);
                        structure_resource.refetch();
                    }
                />
            </Show>
        </div>
    }
}

/// Дерево узлов курса.
///
/// Принимает плоский `Vec<Node>` (результат `fetch_course_structure`),
/// «сплющивает» его в `Vec<(Node, depth)>` и рендерит линейно.
#[component]
fn NodeTree(
    nodes: Vec<Node>,
    on_add_child: impl Fn(Uuid) + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue<Box<dyn Fn>> — чтобы замыкание было Copy и Send + Sync,
    // можно использовать в нескольких местах view! многократно.
    let on_add_child: StoredValue<Box<dyn Fn(Uuid) + Send + Sync + 'static>> =
        StoredValue::new(Box::new(on_add_child));

    // ✅ Один раз «сплющиваем» дерево в плоский список с глубиной.
    // Никакой рекурсии в view! — значит, нет lifetime-проблем.
    let flattened = flatten_tree(nodes);
    let flattened_stored = StoredValue::new(flattened);

    view! {
        <div class="space-y-2">
            {move || {
                flattened_stored.with_value(|list| {
                    list.iter().map(|(node, depth)| {
                        view! {
                            <NodeRow
                                node=node.clone()
                                depth=*depth
                                on_add_child=on_add_child
                            />
                        }
                    }).collect::<Vec<_>>()
                })
            }}
        </div>
    }
}

/// Одна строка дерева (глава или урок).
#[component]
fn NodeRow(
    node: Node,
    depth: usize,
    on_add_child: StoredValue<Box<dyn Fn(Uuid) + Send + Sync + 'static>>,
) -> impl IntoView {
    let is_chapter = node.node_type == NodeType::Chapter;
    let indent_class = format!("ml-{}", depth * 6);
    let node_id = node.id.0;
    let node_href_id = node.id.0.to_string();
    let node_title = node.title.clone();

    // duration_minutes живёт в metadata (JSONB), а не в полях Node
    let duration = node
        .metadata
        .get("duration_minutes")
        .and_then(|v| v.as_u64())
        .map(|d| format!(" • {} мин", d))
        .unwrap_or_default();

    view! {
        <div class=format!("{} mb-2", indent_class)>
            <div class="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg border border-gray-200 dark:border-gray-600 hover:border-blue-300 dark:hover:border-blue-600 transition-colors group">
                // Иконка типа узла
                <div class=move || {
                    if is_chapter {
                        "flex-shrink-0 w-8 h-8 bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 rounded-lg flex items-center justify-center"
                    } else {
                        "flex-shrink-0 w-8 h-8 bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 rounded-lg flex items-center justify-center"
                    }
                }>
                    {if is_chapter { "📁" } else { "📄" }}
                </div>

                // Информация об узле
                <div class="flex-1 min-w-0">
                    <h3 class="font-semibold text-gray-900 dark:text-white truncate">
                        {node_title}
                    </h3>
                    <p class="text-sm text-gray-500 dark:text-gray-400">
                        {if is_chapter { "Глава" } else { "Урок" }}
                        {duration}
                    </p>
                </div>

                // Действия
                <div class="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                        on:click=move |_| on_add_child.with_value(|f| f(node_id))
                        class="p-1.5 text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
                        title="Добавить вложенный элемент"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
                        </svg>
                    </button>
                    <a
                        href=format!("/admin/lessons/{}/edit", node_href_id)
                        class="p-1.5 text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
                        title="Редактировать"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                        </svg>
                    </a>
                </div>
            </div>
        </div>
    }
}

/// «Сплющивает» дерево узлов в плоский список `(Node, depth)`.
///
/// Использует `parent_id` (Adjacency List) для восстановления иерархии,
/// `sort_order` — для порядка среди братьев.
///
/// ltree (`nodes.path`) на клиент не приходит — это деталь хранения в БД.
/// Нам достаточно `parent_id` + `sort_order`, чтобы восстановить дерево.
fn flatten_tree(nodes: Vec<Node>) -> Vec<(Node, usize)> {
    use std::collections::HashMap;

    // Группируем узлы по родителю: HashMap<Option<Uuid>, Vec<Node>>
    let mut by_parent: HashMap<Option<Uuid>, Vec<Node>> = HashMap::new();
    for node in nodes {
        let parent = node.parent_id.map(|p| p.0);
        by_parent.entry(parent).or_default().push(node);
    }

    // Сортируем братьев по sort_order
    for siblings in by_parent.values_mut() {
        siblings.sort_by_key(|n| n.sort_order);
    }

    // Обход в глубину (DFS) с накоплением (Node, depth)
    let mut out: Vec<(Node, usize)> = Vec::new();
    walk(&by_parent, None, 0, &mut out);
    out
}

/// Рекурсивный обход дерева для flatten_tree.
///
/// Здесь рекурсия безопасна: она не возвращает `impl IntoView`,
/// а просто наполняет `Vec`.
fn walk(
    by_parent: &std::collections::HashMap<Option<Uuid>, Vec<Node>>,
    parent: Option<Uuid>,
    depth: usize,
    out: &mut Vec<(Node, usize)>,
) {
    if let Some(children) = by_parent.get(&parent) {
        for node in children {
            out.push((node.clone(), depth));
            walk(by_parent, Some(node.id.0), depth + 1, out);
        }
    }
}

/// Модальное окно добавления главы или урока.
#[component]
fn AddNodeModal(
    course_id: String,
    parent_id: Option<Uuid>,
    on_close: impl Fn() + Send + Sync + 'static,
    on_created: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (Fn + Send + Sync)
    let on_close = StoredValue::new(on_close);
    let on_created = StoredValue::new(on_created);

    let (node_type, set_node_type) = signal(NodeType::Chapter);
    let (title, set_title) = signal(String::new());
    let (content, set_content) = signal(String::new());
    let (duration, set_duration) = signal(0u32);
    let (error, set_error) = signal(None::<String>);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let course_id = course_id.clone();
        spawn_local(async move {
            match create_node(
                course_id,
                parent_id,
                node_type.get(),
                title.get(),
                content.get(),
                duration.get(),
            ).await {
                Ok(_) => on_created.with_value(|f| f()),
                Err(e) => set_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-3xl w-full max-h-[90vh] overflow-y-auto">
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <h3 class="text-2xl font-bold text-gray-900 dark:text-white">
                        {if parent_id.is_some() { "Добавить вложенный элемент" } else { "Добавить элемент" }}
                    </h3>
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                <form on:submit=on_submit class="p-6 space-y-4">
                    <Show when=move || error.get().is_some()>
                        <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg text-sm">
                            {error.get().unwrap()}
                        </div>
                    </Show>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                            "Тип элемента"
                        </label>
                        <div class="flex gap-3">
                            <label class="flex-1 cursor-pointer">
                                <input
                                    type="radio"
                                    name="node_type"
                                    value="chapter"
                                    prop:checked=node_type.get() == NodeType::Chapter
                                    on:change=move |_| set_node_type.set(NodeType::Chapter)
                                    class="sr-only"
                                />
                                <div class=move || {
                                    format!(
                                        "p-4 border-2 rounded-lg text-center transition-all {}",
                                        if node_type.get() == NodeType::Chapter {
                                            "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                                        } else {
                                            "border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600"
                                        }
                                    )
                                }>
                                    <div class="text-2xl mb-1">"📁"</div>
                                    <div class="font-medium text-gray-900 dark:text-white">"Глава"</div>
                                </div>
                            </label>
                            <label class="flex-1 cursor-pointer">
                                <input
                                    type="radio"
                                    name="node_type"
                                    value="lesson"
                                    prop:checked=node_type.get() == NodeType::Lesson
                                    on:change=move |_| set_node_type.set(NodeType::Lesson)
                                    class="sr-only"
                                />
                                <div class=move || {
                                    format!(
                                        "p-4 border-2 rounded-lg text-center transition-all {}",
                                        if node_type.get() == NodeType::Lesson {
                                            "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                                        } else {
                                            "border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600"
                                        }
                                    )
                                }>
                                    <div class="text-2xl mb-1">"📄"</div>
                                    <div class="font-medium text-gray-900 dark:text-white">"Урок"</div>
                                </div>
                            </label>
                        </div>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            "Название *"
                        </label>
                        <input
                            type="text"
                            prop:value=title
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                            required
                        />
                    </div>

                    <Show when=move || node_type.get() == NodeType::Lesson>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                "Содержание урока (Markdown)"
                            </label>
                            <textarea
                                prop:value=content
                                on:input=move |ev| set_content.set(event_target_value(&ev))
                                rows="10"
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 font-mono text-sm"
                                placeholder="# Заголовок&#10;&#10;Текст урока с поддержкой **Markdown**..."
                            ></textarea>
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                "Длительность (минуты)"
                            </label>
                            <input
                                type="number"
                                prop:value=duration
                                on:input=move |ev| {
                                    if let Ok(val) = event_target_value(&ev).parse() {
                                        set_duration.set(val);
                                    }
                                }
                                min="0"
                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500"
                            />
                        </div>
                    </Show>

                    <div class="flex gap-3 pt-4">
                        <button
                            type="submit"
                            class="flex-1 px-4 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
                        >
                            "Создать"
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

#[component]
fn StructureSkeleton() -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6 space-y-3 animate-pulse">
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded-lg ml-6"></div>
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded-lg ml-12"></div>
            <div class="h-16 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
        </div>
    }
}

// ============================================================================
// Server-функции (мок-данные для вёрстки)
// ============================================================================

#[server(GetCourseStructure, "/api")]
pub async fn fetch_course_structure(_course_id: String) -> Result<Vec<Node>, ServerFnError> {
    // TODO: Интеграция с NodeRepository::find_course_tree() из Этапа 5.
    // На этапе вёрстки возвращаем мок-структуру курса:
    // 1 глава + 2 вложенных урока + 1 отдельный урок на верхнем уровне.
    use rust_lms_shared::models::{CourseId, NodeId, TenantId};

    let tenant_id = TenantId::new();
    let course_id = CourseId::new();

    let chapter_id = NodeId::new();
    let lesson2_id = NodeId::new();

    let mock = vec![
        Node {
            id: chapter_id,
            tenant_id,
            parent_id: None,
            node_type: NodeType::Chapter,
            course_id: Some(course_id),
            title: "Глава 1. Введение в Rust".to_string(),
            title_i18n: None,
            description: Some("Знакомство с языком и инструментами".to_string()),
            metadata: serde_json::json!({}),
            sort_order: 1,
            is_archived: false,
            archived_at: None,
        },
        Node {
            id: NodeId::new(),
            tenant_id,
            parent_id: Some(chapter_id),
            node_type: NodeType::Lesson,
            course_id: Some(course_id),
            title: "Урок 1.1. Установка Rust".to_string(),
            title_i18n: None,
            description: None,
            metadata: serde_json::json!({ "duration_minutes": 15 }),
            sort_order: 1,
            is_archived: false,
            archived_at: None,
        },
        Node {
            id: lesson2_id,
            tenant_id,
            parent_id: Some(chapter_id),
            node_type: NodeType::Lesson,
            course_id: Some(course_id),
            title: "Урок 1.2. Первая программа".to_string(),
            title_i18n: None,
            description: None,
            metadata: serde_json::json!({ "duration_minutes": 20 }),
            sort_order: 2,
            is_archived: false,
            archived_at: None,
        },
        Node {
            id: NodeId::new(),
            tenant_id,
            parent_id: Some(lesson2_id),
            node_type: NodeType::Lesson,
            course_id: Some(course_id),
            title: "Тест 1.2.1. Основы Cargo".to_string(),
            title_i18n: None,
            description: None,
            metadata: serde_json::json!({ "duration_minutes": 5 }),
            sort_order: 1,
            is_archived: false,
            archived_at: None,
        },
        Node {
            id: NodeId::new(),
            tenant_id,
            parent_id: None,
            node_type: NodeType::Chapter,
            course_id: Some(course_id),
            title: "Глава 2. Продвинутые темы".to_string(),
            title_i18n: None,
            description: None,
            metadata: serde_json::json!({}),
            sort_order: 2,
            is_archived: false,
            archived_at: None,
        },
    ];

    Ok(mock)
}

#[server(CreateNode, "/api")]
pub async fn create_node(
    _course_id: String,
    parent_id: Option<Uuid>,
    node_type: NodeType,
    title: String,
    content: String,
    duration_minutes: u32,
) -> Result<Node, ServerFnError> {
    // TODO: Интеграция с NodeRepository::create() из Этапа 5.
    // На этапе вёрстки возвращаем мок-узел.
    //
    // Контент и duration_minutes кладём в metadata (JSONB),
    // т.к. в shared::Node отдельных полей для них нет.
    use rust_lms_shared::models::{CourseId, NodeId, TenantId};

    let mut metadata = serde_json::json!({});
    if !content.is_empty() {
        metadata["content"] = serde_json::Value::String(content);
    }
    if duration_minutes > 0 {
        metadata["duration_minutes"] = serde_json::Value::Number(duration_minutes.into());
    }

    let node = Node {
        id: NodeId::new(),
        tenant_id: TenantId::new(),
        parent_id: parent_id.map(NodeId),
        node_type,
        course_id: Some(CourseId::new()),
        title,
        title_i18n: None,
        description: None,
        metadata,
        sort_order: 0,
        is_archived: false,
        archived_at: None,
    };

    Ok(node)
}