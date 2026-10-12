// crates/client/src/student/pages/player.rs
//! Плеер уроков с интеграцией прогресса (Этап 9) и тестов (Этап 10).
//!
//! Отображает контент урока, дерево курса, позволяет отмечать уроки как завершённые
//! и проходить встроенные тесты с автоматическим обновлением прогресса.

use leptos::prelude::*;
use rust_lms_shared::models::{Course, LessonProgress, Node, NodeType};
// use rust_lms_ui::components::link::Link;

/// Компонент плеера уроков.
#[component]
pub fn PlayerPage() -> impl IntoView {
    // Получаем ID курса из URL-параметра
    let params = use_params_map();
    let course_id = move || params.with(|p| p.get("course_id").cloned().unwrap_or_default());
    
    // Ресурсы для загрузки данных
    let course_resource = create_resource(course_id, fetch_course_with_tree);
    let progress_resource = create_resource(course_id, fetch_user_progress);
    let current_lesson_resource = create_resource(course_id, fetch_current_lesson);

    // Сигналы для состояния UI
    let (sidebar_open, set_sidebar_open) = signal(true);
    let (show_test_modal, set_show_test_modal) = signal(false);

    view! {
        <div class="flex h-screen bg-gray-50 dark:bg-gray-900">
            // Боковая панель с деревом курса
            <Sidebar
                course_resource=course_resource
                current_lesson_resource=current_lesson_resource
                open=sidebar_open
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
                                            on_complete=move || {
                                                // TODO: Вызов API для отметки урока как завершённого
                                                // POST /api/v1/progress
                                            }
                                            on_start_test=move || set_show_test_modal.set(true)
                                        />
                                    }.into_view(),
                                    Ok(None) => view! {
                                        <div class="flex items-center justify-center h-full">
                                            <p class="text-gray-500 dark:text-gray-400">
                                                "Урок не найден"
                                            </p>
                                        </div>
                                    }.into_view(),
                                    Err(e) => view! {
                                        <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg m-4">
                                            <p class="font-semibold">"Ошибка загрузки урока"</p>
                                            <p class="text-sm">{e.to_string()}</p>
                                        </div>
                                    }.into_view(),
                                }
                            })
                        }}
                    </Suspense>
                </main>
            </div>

            // Модальное окно прохождения теста (Этап 10)
            <Show when=move || show_test_modal.get()>
                <TestModal
                    lesson_id=current_lesson_resource.get().unwrap_or(None).and_then(|l| l.map(|n| n.id))
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
    course_resource: Resource<Uuid, Result<CourseWithTree, ServerFnError>>,
    current_lesson_resource: Resource<Uuid, Result<Option<Node>, ServerFnError>>,
    open: Signal<bool>,
    on_toggle: impl Fn() + 'static,
) -> impl IntoView {
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
                    on:click=move |_| on_toggle()
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
                                        current_lesson=current_lesson_resource.get().unwrap_or(None).unwrap_or(None)
                                    />
                                }.into_view(),
                                Err(e) => view! {
                                    <p class="p-4 text-red-600 dark:text-red-400">{e.to_string()}</p>
                                }.into_view(),
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
    course_resource: Resource<Uuid, Result<CourseWithTree, ServerFnError>>,
    progress_resource: Resource<Uuid, Result<UserProgress, ServerFnError>>,
    on_toggle_sidebar: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-4">
                    <button
                        on:click=move |_| on_toggle_sidebar()
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
                                    }.into_view(),
                                    Err(_) => view! { <div></div> }.into_view(),
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
                                    let percent = (progress.completed_weight * 100.0).round() as u32;
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
                                    }.into_view()
                                }
                                Err(_) => view! { <div></div> }.into_view(),
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
    on_complete: impl Fn() + 'static,
    on_start_test: impl Fn() + 'static,
) -> impl IntoView {
    let is_completed = lesson.progress.as_ref().map(|p| p.status == "completed").unwrap_or(false);

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
                    {lesson.duration_minutes.map(|d| format!("{} мин", d)).unwrap_or_else(|| "—".to_string())}
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
                // TODO: Рендеринг Markdown из lesson.content
                <div inner_html=lesson.content.clone().unwrap_or_default()></div>
            </div>

            // Панель действий
            <div class="flex items-center justify-between gap-4 p-6 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700">
                <div>
                    <Show when=move || !is_completed>
                        <button
                            on:click=move |_| on_complete()
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
                <Show when=move || lesson.has_assessment.unwrap_or(false)>
                    <button
                        on:click=move |_| on_start_test()
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
    on_close: impl Fn() + 'static,
    on_complete: impl Fn() + 'static,
) -> impl IntoView {
    let test_resource = create_resource(
        move || lesson_id,
        |lesson_id| async move {
            if let Some(id) = lesson_id {
                fetch_test_questions(id).await
            } else {
                Err(ServerFnError::new("Lesson ID not found"))
            }
        }
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
                        on:click=move |_| on_close()
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
                                                on_complete();
                                            }
                                        />
                                    }.into_view(),
                                    Err(e) => view! {
                                        <div class="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                            <p class="font-semibold">"Ошибка загрузки теста"</p>
                                            <p class="text-sm">{e.to_string()}</p>
                                        </div>
                                    }.into_view(),
                                }
                            })
                        }}
                    </Suspense>
                </div>
            </div>
        </div>
    }
}

// Вспомогательные компоненты и сервер-функции (упрощённые для краткости)

#[component]
fn CourseTree(nodes: Vec<Node>, current_lesson: Option<Node>) -> impl IntoView {
    // TODO: Рекурсивный рендеринг дерева с использованием ltree path
    view! {
        <nav class="p-4">
            {nodes.into_iter().map(|node| {
                let is_current = current_lesson.as_ref().map(|c| c.id == node.id).unwrap_or(false);
                view! {
                    <a
                        href=format!("/student/courses/{}/lessons/{}", node.course_id, node.id)
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
                        {node.title}
                    </a>
                }
            }).collect::<Vec<_>>()}
        </nav>
    }
}

#[component]
fn TestQuestions(questions: Vec<Question>, on_submit: impl Fn() + 'static) -> impl IntoView {
    // TODO: Реализация интерфейса вопросов из Этапа 10
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            on_submit();
        }>
            {questions.into_iter().enumerate().map(|(idx, q)| {
                view! {
                    <div class="mb-8 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                        <p class="font-semibold text-gray-900 dark:text-white mb-3">
                            {idx + 1} ". " {q.title}
                        </p>
                        // TODO: Рендеринг вариантов ответа в зависимости от q.question_type
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

// Сервер-функции (заглушки для демонстрации)

#[server(GetCourseWithTree, "/api")]
pub async fn fetch_course_with_tree(course_id: String) -> Result<CourseWithTree, ServerFnError> {
    // TODO: Интеграция с NodeRepository::find_course_tree() из Этапа 5
    Err(ServerFnError::new("Not implemented"))
}

#[server(GetUserProgress, "/api")]
pub async fn fetch_user_progress(course_id: String) -> Result<UserProgress, ServerFnError> {
    // TODO: Интеграция с ProgressService::get_user_course_progress() из Этапа 9
    Err(ServerFnError::new("Not implemented"))
}

#[server(GetCurrentLesson, "/api")]
pub async fn fetch_current_lesson(course_id: String) -> Result<Option<Node>, ServerFnError> {
    // TODO: Получить текущий урок (последний незавершённый или первый)
    Err(ServerFnError::new("Not implemented"))
}

#[server(GetTestQuestions, "/api")]
pub async fn fetch_test_questions(lesson_id: Uuid) -> Result<TestWithQuestions, ServerFnError> {
    // TODO: Интеграция с QuestionRepository из Этапа 10
    Err(ServerFnError::new("Not implemented"))
}

// DTO структуры (должны быть в crates/shared/src/models/)

pub struct CourseWithTree {
    pub title: String,
    pub tree: Vec<Node>,
}

pub struct UserProgress {
    pub completed_weight: f64,
}

pub struct Question {
    pub id: Uuid,
    pub title: String,
    pub question_type: String,
    pub options: Option<Vec<String>>,
}

pub struct TestWithQuestions {
    pub questions: Vec<Question>,
}