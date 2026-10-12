// crates/client/src/student/pages/player.rs
//! Плеер уроков с интеграцией прогресса (Этап 9) и тестов (Этап 10).
//!
//! Отображает контент урока, дерево курса, позволяет отмечать уроки как завершённые
//! и проходить встроенные тесты с автоматическим обновлением прогресса.
//!
//! ⚠️ На этапе вёрстки БД не подключена — server-функции возвращают мок-данные.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_lms_shared::models::{
    CourseProgressSummary, LessonProgress, LessonStatus, Node, NodeType, Question, QuestionType,
};
// use rust_lms_ui::components::link::Link;

/// Компонент плеера уроков.
#[component]
pub fn PlayerPage() -> impl IntoView {
    // Получаем ID курса из URL-параметра
    let params = use_params_map();
    let course_id = move || {
        params
            .with(|p| p.get("course_id").map(|s| s.to_string()))
            .unwrap_or_default()
    };

    // ✅ Leptos 0.7/0.8: Resource::new вместо create_resource.
    // Ресурсы для загрузки данных (параметризованы String — ID курса из URL).
    let course_resource = Resource::new(course_id, |id| fetch_course_with_tree(id));
    let progress_resource = Resource::new(course_id, |id| fetch_user_progress(id));
    let current_lesson_resource = Resource::new(course_id, |id| fetch_current_lesson(id));

    // Сигналы для состояния UI
    let (sidebar_open, set_sidebar_open) = signal(true);
    let (show_test_modal, set_show_test_modal) = signal(false);

    view! {
        <div class="flex h-screen bg-gray-50 dark:bg-gray-900">
            // Боковая панель с деревом курса
            <Sidebar
                course_resource=course_resource
                current_lesson_resource=current_lesson_resource
                // ✅ ReadSignal<bool> → Signal<bool> через .into()
                open=sidebar_open.into()
                on_toggle=move || set_sidebar_open.update(|v| *v = !*v)
            />

            // Основная область контента
            <div class="flex-1 flex flex-col overflow-hidden">
                // Верхняя панель с навигацией и прогрессом
                <TopBar
                    course_resource=course_resource
                    progress_resource=progress_resource
                    on_toggle_sidebar=move || set_sidebar_open.update(|v| *v = !*v)
                />

                // Контент урока
                <main class="flex-1 overflow-y-auto">
                    <Suspense fallback=LessonSkeleton>
                        {move || {
                            current_lesson_resource.get().map(|result| {
                                match result {
                                    Ok(Some(lesson)) => view! {
                                        <LessonContent
                                            lesson=lesson
                                            // На этапе вёрстки прогресс не связан с уроком —
                                            // передаём None, is_completed будет false.
                                            progress=None
                                            on_complete=move || {
                                                // TODO: Вызов API для отметки урока как завершённого
                                                // POST /api/v1/progress
                                            }
                                            on_start_test=move || set_show_test_modal.set(true)
                                        />
                                    }.into_any(),
                                    Ok(None) => view! {
                                        <div class="flex items-center justify-center h-full">
                                            <p class="text-gray-500 dark:text-gray-400">
                                                "Урок не найден"
                                            </p>
                                        </div>
                                    }.into_any(),
                                    Err(e) => view! {
                                        <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg m-4">
                                            <p class="font-semibold">"Ошибка загрузки урока"</p>
                                            <p class="text-sm">{e.to_string()}</p>
                                        </div>
                                    }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </main>
            </div>

            // Модальное окно прохождения теста (Этап 10)
            <Show when=move || show_test_modal.get()>
                <TestModal
                    lesson_id=current_lesson_resource
                        .get()
                        .and_then(|r| r.ok())
                        .flatten()
                        .map(|n| n.id.0)
                    on_close=move || set_show_test_modal.set(false)
                    on_complete=move || {
                        set_show_test_modal.set(false);
                        // TODO: Перезагрузить прогресс после успешного прохождения теста
                    }
                />
            </Show>
        </div>
    }
}

/// Боковая панель с деревом курса (использует ltree из Этапа 5).
#[component]
fn Sidebar(
    course_resource: Resource<Result<CourseWithTree, ServerFnError>>,
    current_lesson_resource: Resource<Result<Option<Node>, ServerFnError>>,
    open: Signal<bool>,
    on_toggle: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue делает замыкание Copy, чтобы можно было использовать в view! многократно
    let on_toggle = StoredValue::new(on_toggle);

    view! {
        <aside
            class=move || {
                format!(
                    "fixed inset-y-0 left-0 z-50 w-80 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 transform transition-transform duration-300 ease-in-out {}",
                    if open.get() { "translate-x-0" } else { "-translate-x-full" }
                )
            }
        >
            <div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
                <h2 class="text-lg font-semibold text-gray-900 dark:text-white">
                    // i18n: t!("player.sidebar.title")
                    "Содержание курса"
                </h2>
                <button
                    on:click=move |_| on_toggle.with_value(|f| f())
                    class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                >
                    <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                </button>
            </div>

            <div class="overflow-y-auto h-full pb-20">
                <Suspense fallback=SidebarSkeleton>
                    {move || {
                        course_resource.get().map(|result| {
                            match result {
                                Ok(course) => view! {
                                    <CourseTree
                                        nodes=course.tree
                                        current_lesson=current_lesson_resource
                                            .get()
                                            .and_then(|r| r.ok())
                                            .flatten()
                                    />
                                }.into_any(),
                                Err(e) => view! {
                                    <p class="p-4 text-red-600 dark:text-red-400">{e.to_string()}</p>
                                }.into_any(),
                            }
                        })
                    }}
                </Suspense>
            </div>
        </aside>
    }
}

/// Верхняя панель с информацией о курсе и прогрессе.
#[component]
fn TopBar(
    course_resource: Resource<Result<CourseWithTree, ServerFnError>>,
    progress_resource: Resource<Result<CourseProgressSummary, ServerFnError>>,
    on_toggle_sidebar: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue — чтобы замыкание можно было использовать в Fn-контексте
    let on_toggle_sidebar = StoredValue::new(on_toggle_sidebar);

    view! {
        <header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-4">
                    <button
                        on:click=move |_| on_toggle_sidebar.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path>
                        </svg>
                    </button>

                    <Suspense fallback=move || view! { <div class="h-6 w-48 bg-gray-200 dark:bg-gray-700 rounded animate-pulse"></div> }>
                        {move || {
                            course_resource.get().map(|result| {
                                match result {
                                    Ok(course) => view! {
                                        <h1 class="text-xl font-bold text-gray-900 dark:text-white">
                                            {course.title}
                                        </h1>
                                    }.into_any(),
                                    Err(_) => view! { <div></div> }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>

                // Индикатор прогресса курса
                <Suspense fallback=move || view! { <div class="h-8 w-32 bg-gray-200 dark:bg-gray-700 rounded animate-pulse"></div> }>
                    {move || {
                        progress_resource.get().map(|result| {
                            match result {
                                Ok(progress) => {
                                    let percent = (progress.progress * 100.0).round() as u32;
                                    view! {
                                        <div class="flex items-center gap-3">
                                            <div class="text-sm font-medium text-gray-700 dark:text-gray-300">
                                                // i18n: t!("player.progress.label")
                                                "Прогресс:"
                                            </div>
                                            <div class="flex items-center gap-2">
                                                <div class="w-32 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                                                    <div
                                                        class="bg-blue-600 h-2 rounded-full transition-all duration-500"
                                                        style:width=format!("{}%", percent)
                                                    ></div>
                                                </div>
                                                <span class="text-sm font-semibold text-gray-900 dark:text-white">
                                                    {percent} "%"
                                                </span>
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                                Err(_) => view! { <div></div> }.into_any(),
                            }
                        })
                    }}
                </Suspense>
            </div>
        </header>
    }
}

/// Контент урока с кнопками действий.
#[component]
fn LessonContent(
    lesson: Node,
    /// Прогресс по уроку (None — если ещё не начат).
    progress: Option<LessonProgress>,
    on_complete: impl Fn() + Send + Sync + 'static,
    on_start_test: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ Оборачиваем callbacks в StoredValue, чтобы:
    // 1) замыкания в view! были Fn (не FnOnce),
    // 2) выполнялось требование Send + Sync для Leptos 0.8.
    let on_complete = StoredValue::new(on_complete);
    let on_start_test = StoredValue::new(on_start_test);

    // ✅ Проверяем завершение по LessonStatus (а не по строке)
    let is_completed = progress
        .as_ref()
        .map(|p| p.status == LessonStatus::Completed)
        .unwrap_or(false);

    // ✅ Данные, которых нет в Node напрямую, достаём из metadata (JSONB).
    let duration = lesson
        .metadata
        .get("duration_minutes")
        .and_then(|v| v.as_i64())
        .map(|d| format!("{} мин", d))
        .unwrap_or_else(|| "—".to_string());

    let content = lesson
        .metadata
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("<p class=\"text-gray-500\">Содержимое урока будет загружено после подключения БД.</p>")
        .to_string();

    let has_assessment = lesson
        .metadata
        .get("has_assessment")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    view! {
        <article class="max-w-4xl mx-auto px-6 py-8">
            // Заголовок урока
            <h2 class="text-3xl font-bold text-gray-900 dark:text-white mb-4">
                {lesson.title.clone()}
            </h2>

            // Метаданные урока
            <div class="flex items-center gap-4 text-sm text-gray-600 dark:text-gray-400 mb-8">
                <span class="flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    // i18n: t!("player.lesson.duration")
                    {duration}
                </span>
                <Show when=move || is_completed>
                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300">
                        // i18n: t!("player.lesson.completed_badge")
                        "✓ Завершён"
                    </span>
                </Show>
            </div>

            // Контент урока (Markdown, изображения, видео)
            <div class="prose prose-lg dark:prose-invert max-w-none mb-12">
                // TODO: Рендеринг Markdown из lesson.metadata.content
                <div inner_html=content></div>
            </div>

            // Панель действий
            <div class="flex items-center justify-between gap-4 p-6 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700">
                <div>
                    <Show when=move || !is_completed>
                        <button
                            on:click=move |_| on_complete.with_value(|f| f())
                            class="px-6 py-3 bg-green-600 text-white font-medium rounded-lg hover:bg-green-700 active:bg-green-800 transition-colors duration-200 shadow-sm"
                        >
                            // i18n: t!("player.lesson.complete_button")
                            "✓ Отметить как завершённый"
                        </button>
                    </Show>
                    <Show when=move || is_completed>
                        <div class="flex items-center gap-2 text-green-600 dark:text-green-400">
                            <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
                            </svg>
                            // i18n: t!("player.lesson.completed_message")
                            <span class="font-medium">"Урок завершён"</span>
                        </div>
                    </Show>
                </div>

                // Кнопка прохождения теста (если урок содержит тест)
                <Show when=move || has_assessment>
                    <button
                        on:click=move |_| on_start_test.with_value(|f| f())
                        class="px-6 py-3 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 active:bg-blue-800 transition-colors duration-200 shadow-sm"
                    >
                        // i18n: t!("player.lesson.start_test_button")
                        "📝 Пройти тест"
                    </button>
                </Show>
            </div>
        </article>
    }
}

/// Модальное окно прохождения теста (Этап 10).
#[component]
fn TestModal(
    lesson_id: Option<Uuid>,
    on_close: impl Fn() + Send + Sync + 'static,
    on_complete: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callbacks (см. пояснение в LessonContent)
    let on_close = StoredValue::new(on_close);
    let on_complete = StoredValue::new(on_complete);

    // ✅ Leptos 0.7/0.8: Resource::new
    let test_resource = Resource::new(
        move || lesson_id,
        |lesson_id| async move {
            if let Some(id) = lesson_id {
                fetch_test_questions(id).await
            } else {
                Err(ServerFnError::new("Lesson ID not found"))
            }
        },
    );

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-3xl w-full max-h-[90vh] overflow-hidden flex flex-col">
                // Заголовок модалки
                <div class="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-700">
                    <h3 class="text-2xl font-bold text-gray-900 dark:text-white">
                        // i18n: t!("player.test.modal_title")
                        "Прохождение теста"
                    </h3>
                    <button
                        on:click=move |_| on_close.with_value(|f| f())
                        class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <svg class="w-5 h-5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>

                // Контент теста
                <div class="flex-1 overflow-y-auto p-6">
                    <Suspense fallback=TestSkeleton>
                        {move || {
                            test_resource.get().map(|result| {
                                match result {
                                    Ok(test) => view! {
                                        <TestQuestions
                                            questions=test.questions
                                            on_submit=move || {
                                                on_complete.with_value(|f| f());
                                            }
                                        />
                                    }.into_any(),
                                    Err(e) => view! {
                                        <div class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                            <p class="font-semibold">"Ошибка загрузки теста"</p>
                                            <p class="text-sm">{e.to_string()}</p>
                                        </div>
                                    }.into_any(),
                                }
                            })
                        }}
                    </Suspense>
                </div>
            </div>
        </div>
    }
}

// Вспомогательные компоненты

/// Дерево курса (главы, темы, уроки).
#[component]
fn CourseTree(nodes: Vec<Node>, current_lesson: Option<Node>) -> impl IntoView {
    // TODO: Рекурсивный рендеринг дерева с использованием ltree path
    view! {
        <nav class="p-4">
            {nodes.into_iter().map(|node| {
                let is_current = current_lesson
                    .as_ref()
                    .map(|c| c.id == node.id)
                    .unwrap_or(false);
                // ✅ course_id — Option<CourseId>, извлекаем UUID или пустую строку
                let course_id_str = node
                    .course_id
                    .map(|c| c.0.to_string())
                    .unwrap_or_default();
                let node_id_str = node.id.0.to_string();
                let href = format!("/student/courses/{}/lessons/{}", course_id_str, node_id_str);
                let node_title = node.title.clone();
                let node_type = node.node_type;

                view! {
                    <a
                        href=href
                        class=move || {
                            format!(
                                "block p-3 rounded-lg mb-2 transition-colors {}",
                                if is_current {
                                    "bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 font-medium"
                                } else {
                                    "hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300"
                                }
                            )
                        }
                    >
                        // Индикатор типа узла (chapter/topic/lesson)
                        <span class="inline-block w-4 text-xs text-gray-400 mr-1">
                            {match node_type {
                                NodeType::Chapter => "📖",
                                NodeType::Topic => "📂",
                                NodeType::Lesson => "📄",
                                _ => "•",
                            }}
                        </span>
                        {node_title}
                    </a>
                }
            }).collect::<Vec<_>>()}
        </nav>
    }
}

/// Список вопросов теста с формой.
#[component]
fn TestQuestions(
    questions: Vec<Question>,
    on_submit: impl Fn() + Send + Sync + 'static,
) -> impl IntoView {
    // ✅ StoredValue для callback
    let on_submit = StoredValue::new(on_submit);

    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            on_submit.with_value(|f| f());
        }>
            {questions.into_iter().enumerate().map(|(idx, q)| {
                let title = q.title.clone();
                let q_type = q.question_type;
                let options = q.options.clone();
                view! {
                    <div class="mb-8 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                        <p class="font-semibold text-gray-900 dark:text-white mb-3">
                            {idx + 1} ". " {title}
                        </p>
                        // Рендеринг вариантов ответа в зависимости от типа вопроса
                        {match q_type {
                            QuestionType::MultipleChoice => {
                                let opts = options.unwrap_or_default();
                                view! {
                                    <div class="space-y-2">
                                        {opts.into_iter().map(|opt| {
                                            let text = opt.text.clone();
                                            view! {
                                                <label class="flex items-center gap-2 cursor-pointer">
                                                    <input type="radio" name=format!("q_{}", idx) class="w-4 h-4" />
                                                    <span class="text-gray-700 dark:text-gray-300">{text}</span>
                                                </label>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            },
                            QuestionType::TrueFalse => view! {
                                <div class="space-y-2">
                                    <label class="flex items-center gap-2 cursor-pointer">
                                        <input type="radio" name=format!("q_{}", idx) class="w-4 h-4" />
                                        <span class="text-gray-700 dark:text-gray-300">"Верно"</span>
                                    </label>
                                    <label class="flex items-center gap-2 cursor-pointer">
                                        <input type="radio" name=format!("q_{}", idx) class="w-4 h-4" />
                                        <span class="text-gray-700 dark:text-gray-300">"Неверно"</span>
                                    </label>
                                </div>
                            }.into_any(),
                            QuestionType::ShortAnswer => view! {
                                <input
                                    type="text"
                                    class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                                    placeholder="Ваш ответ"
                                />
                            }.into_any(),
                            QuestionType::LongAnswer => view! {
                                <textarea
                                    rows="4"
                                    class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                                    placeholder="Развёрнутый ответ"
                                ></textarea>
                            }.into_any(),
                        }}
                    </div>
                }
            }).collect::<Vec<_>>()}
            <button
                type="submit"
                class="w-full px-6 py-3 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
            >
                // i18n: t!("player.test.submit_button")
                "Завершить тест"
            </button>
        </form>
    }
}

// Скелетоны загрузки
#[component]
fn LessonSkeleton() -> impl IntoView {
    view! {
        <div class="max-w-4xl mx-auto px-6 py-8 animate-pulse">
            <div class="h-8 bg-gray-200 dark:bg-gray-700 rounded w-3/4 mb-4"></div>
            <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/4 mb-8"></div>
            <div class="space-y-3">
                <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
                <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
                <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-5/6"></div>
            </div>
        </div>
    }
}

#[component]
fn SidebarSkeleton() -> impl IntoView {
    view! {
        <div class="p-4 space-y-2 animate-pulse">
            <div class="h-10 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-10 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-10 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </div>
    }
}

#[component]
fn TestSkeleton() -> impl IntoView {
    view! {
        <div class="space-y-6 animate-pulse">
            <div class="h-20 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-20 bg-gray-200 dark:bg-gray-700 rounded"></div>
            <div class="h-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
        </div>
    }
}

// ============================================================================
// DTO и сервер-функции
// ============================================================================

/// DTO курса с деревом узлов.
///
/// Локальный DTO для плеера: объединяет метаданные курса (title) и дерево узлов.
/// ⚠️ Не путать с `Course` из shared — там нет дерева.
/// При подключении БД — заполнять из `NodeRepository::find_course_tree()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseWithTree {
    /// Заголовок курса.
    pub title: String,
    /// Дерево узлов курса (главы, темы, уроки).
    pub tree: Vec<Node>,
}

/// DTO теста с вопросами.
///
/// Локальный DTO для плеера: тест + список вопросов.
/// При подключении БД — заполнять из `QuestionRepository`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWithQuestions {
    /// Вопросы теста.
    pub questions: Vec<Question>,
}

// ----------------------------------------------------------------------------
// Server-функции (мок-данные для вёрстки)
// ----------------------------------------------------------------------------

#[server(GetCourseWithTree, "/api")]
pub async fn fetch_course_with_tree(_course_id: String) -> Result<CourseWithTree, ServerFnError> {
    // ✅ Локальные импорты — только для server-части (иначе unused_imports на клиенте)
    use rust_lms_shared::models::{CourseId, NodeId, TenantId};

    // TODO: Интеграция с NodeRepository::find_course_tree() из Этапа 5
    // На этапе вёрстки возвращаем мок-данные.

    let course_uuid = Uuid::new_v4();
    let course_id_typed = CourseId(course_uuid);
    let tenant_id = TenantId::new();

    // Мок-дерево: 1 глава + 2 урока
    let chapter_id = NodeId::new();
    let tree = vec![
        Node {
            id: chapter_id,
            tenant_id,
            parent_id: None,
            node_type: NodeType::Chapter,
            course_id: Some(course_id_typed),
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
            course_id: Some(course_id_typed),
            title: "Урок 1.1. Установка Rust".to_string(),
            title_i18n: None,
            description: None,
            metadata: serde_json::json!({
                "duration_minutes": 15,
                "content": "<p>В этом уроке мы установим <strong>rustup</strong> и настроим окружение.</p><p>Команда: <code>curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh</code></p>",
                "has_assessment": false
            }),
            sort_order: 1,
            is_archived: false,
            archived_at: None,
        },
        Node {
            id: NodeId::new(),
            tenant_id,
            parent_id: Some(chapter_id),
            node_type: NodeType::Lesson,
            course_id: Some(course_id_typed),
            title: "Урок 1.2. Первая программа".to_string(),
            title_i18n: None,
            description: None,
            metadata: serde_json::json!({
                "duration_minutes": 20,
                "content": "<p>Напишем классический <code>Hello, world!</code> и разберём структуру проекта Cargo.</p>",
                "has_assessment": true
            }),
            sort_order: 2,
            is_archived: false,
            archived_at: None,
        },
    ];

    Ok(CourseWithTree {
        title: "Демо-курс: Основы Rust".to_string(),
        tree,
    })
}

#[server(GetUserProgress, "/api")]
pub async fn fetch_user_progress(_course_id: String) -> Result<CourseProgressSummary, ServerFnError> {
    // ✅ Локальные импорты
    use rust_lms_shared::models::{CourseId, UserId};

    // TODO: Интеграция с ProgressService::get_user_course_progress() из Этапа 9
    // На этапе вёрстки возвращаем мок-данные (45% прогресса).

    Ok(CourseProgressSummary {
        course_id: CourseId::new(),
        user_id: UserId::new(),
        progress: 0.45,
        status: "active".to_string(),
        completed_at: None,
        completed_lessons_count: 2,
        total_lessons_count: 5,
        completed_lessons_weight: 0.45,
        total_lessons_weight: 1.0,
    })
}

#[server(GetCurrentLesson, "/api")]
pub async fn fetch_current_lesson(_course_id: String) -> Result<Option<Node>, ServerFnError> {
    // ✅ Локальные импорты
    use rust_lms_shared::models::{CourseId, NodeId, TenantId};

    // TODO: Получить текущий урок (последний незавершённый или первый)
    // На этапе вёрстки возвращаем мок-урок.

    let tenant_id = TenantId::new();
    let course_id_typed = CourseId::new();

    Ok(Some(Node {
        id: NodeId::new(),
        tenant_id,
        parent_id: None,
        node_type: NodeType::Lesson,
        course_id: Some(course_id_typed),
        title: "Урок 1.2. Первая программа".to_string(),
        title_i18n: None,
        description: Some("Пишем Hello, world!".to_string()),
        metadata: serde_json::json!({
            "duration_minutes": 20,
            "content": "<p>Напишем классический <code>Hello, world!</code> и разберём структуру проекта Cargo.</p><p>Создайте проект: <code>cargo new hello_world</code></p>",
            "has_assessment": true
        }),
        sort_order: 2,
        is_archived: false,
        archived_at: None,
    }))
}

#[server(GetTestQuestions, "/api")]
pub async fn fetch_test_questions(lesson_id: Uuid) -> Result<TestWithQuestions, ServerFnError> {
    // ✅ Локальные импорты
    use rust_lms_shared::models::{AnswerOption, CourseId, QuestionId, TenantId};

    // TODO: Интеграция с QuestionRepository из Этапа 10
    // На этапе вёрстки возвращаем мок-тест с 3 вопросами.
    //
    // NB: lesson_id пока не используется, но сохранён в сигнатуре —
    // при подключении БД по нему будем искать вопросы.

    let _ = lesson_id;

    let tenant_id = TenantId::new();
    let course_id = CourseId::new();
    let now = chrono::Utc::now();

    let questions = vec![
        Question {
            // ✅ QuestionId::new() доступен после патча shared
            id: QuestionId::new(),
            tenant_id,
            course_id,
            title: "Какой инструмент управляет версиями Rust?".to_string(),
            description: None,
            question_type: QuestionType::MultipleChoice,
            options: Some(vec![
                AnswerOption { text: "rustup".to_string(), is_correct: true },
                AnswerOption { text: "cargo".to_string(), is_correct: false },
                AnswerOption { text: "clippy".to_string(), is_correct: false },
                AnswerOption { text: "rustc".to_string(), is_correct: false },
            ]),
            correct_answer: None,
            points: 1,
            order: 1,
            created_at: now,
            updated_at: now,
        },
        Question {
            id: QuestionId::new(),
            tenant_id,
            course_id,
            title: "Rust — это язык с автоматическим управлением памятью (GC).".to_string(),
            description: None,
            question_type: QuestionType::TrueFalse,
            options: None,
            correct_answer: Some("false".to_string()),
            points: 1,
            order: 2,
            created_at: now,
            updated_at: now,
        },
        Question {
            id: QuestionId::new(),
            tenant_id,
            course_id,
            title: "Какая команда создаёт новый Cargo-проект?".to_string(),
            description: None,
            question_type: QuestionType::ShortAnswer,
            options: None,
            correct_answer: Some("cargo new".to_string()),
            points: 1,
            order: 3,
            created_at: now,
            updated_at: now,
        },
    ];

    Ok(TestWithQuestions { questions })
}