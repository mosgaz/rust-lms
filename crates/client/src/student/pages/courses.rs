// crates/client/src/student/pages/courses.rs
//! Страница "Мои курсы" в личном кабинете студента.
//!
//! Отображает список курсов, на которые зачислен пользователь,
//! с индикаторами прогресса (Этап 9) и кнопками для продолжения обучения.

use leptos::prelude::*;
use rust_lms_shared::models::CourseEnrollment;

/// Компонент страницы списка курсов студента.
#[component]
pub fn CoursesPage() -> impl IntoView {
    // ✅ Leptos 0.7: Resource::new вместо create_resource
    let courses_resource = Resource::new(
        || (),
        |_| fetch_enrolled_courses_with_progress(),
    );

    view! {
        <div class="container mx-auto px-4 py-8 max-w-7xl">
            <h1 class="text-3xl font-bold mb-2 text-gray-900 dark:text-white">
                "Мои курсы"
            </h1>
            <p class="text-gray-600 dark:text-gray-400 mb-8">
                "Отслеживайте свой прогресс и продолжайте обучение"
            </p>

            <Suspense fallback=move || {
                view! {
                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                        // Скелетон загрузки (3 карточки)
                        <SkeletonCard />
                        <SkeletonCard />
                        <SkeletonCard />
                    </div>
                }
            }>
                {move || {
                    courses_resource.get().map(|result| {
                        match result {
                            Ok(enrollments) => {
                                if enrollments.is_empty() {
                                    view! {
                                        <div class="text-center py-16 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-dashed border-gray-300 dark:border-gray-700">
                                            // i18n: t!("student.courses.empty_state")
                                            <p class="text-gray-500 dark:text-gray-400 mb-4 text-lg">
                                                "Вы пока не зачислены ни на один курс."
                                            </p>
                                            <a
                                                href="/catalog"
                                                class="inline-flex items-center px-5 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors duration-200 shadow-sm"
                                            >
                                                "Перейти в каталог курсов"
                                            </a>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                                            {enrollments.into_iter().map(|enrollment| {
                                                view! { <CourseCard enrollment=enrollment /> }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }
                            Err(e) => view! {
                                <div class="p-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 rounded-lg">
                                    // i18n: t!("student.courses.error_loading")
                                    <p class="font-semibold mb-1">"Ошибка загрузки курсов"</p>
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

/// Карточка отдельного курса с индикатором прогресса.
#[component]
fn CourseCard(enrollment: CourseEnrollment) -> impl IntoView {
    // ✅ Используем существующее поле progress (f64, 0.0 – 1.0)
    let progress_ratio = enrollment.progress;
    let progress_percent = (progress_ratio * 100.0).round() as u32;

    // Определяем статус и цвет прогресс-бара
    let (progress_color, status_text) = if progress_percent >= 100 {
        ("bg-green-600", "Завершён")
    } else if progress_percent > 0 {
        ("bg-blue-600", "В процессе")
    } else {
        ("bg-gray-400", "Не начат")
    };

    // ✅ course_id — обёртка CourseId(Uuid), для URL берём .0
    let course_id = enrollment.course_id.0;

    view! {
        <div class="group bg-white dark:bg-gray-800 rounded-xl shadow-sm hover:shadow-md transition-all duration-300 border border-gray-200 dark:border-gray-700 overflow-hidden flex flex-col">
            <div class="p-6 flex-grow">
                <div class="flex justify-between items-start mb-3">
                    // i18n: динамический статус курса
                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200">
                        {status_text}
                    </span>
                    // i18n: t!("student.courses.progress_label")
                    <span class="text-sm font-semibold text-gray-900 dark:text-white">
                        {progress_percent} "%"
                    </span>
                </div>

                // ⚠️ В DTO нет title/description — показываем course_id как заглушку.
                // Если нужны реальные title/description — их надо тянуть отдельным запросом
                // (или расширить server-функцию, вернув кортеж/новый DTO).
                <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-2 line-clamp-2 group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                    {format!("Курс {}", course_id)}
                </h3>

                <p class="text-gray-600 dark:text-gray-300 text-sm mb-6 line-clamp-3">
                    "Описание отсутствует"
                </p>

                // Индикатор прогресса (Progress Bar)
                <div class="mb-2">
                    <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                        <div
                            class=format!("{} h-2 rounded-full transition-all duration-700 ease-out", progress_color)
                            style:width=format!("{}%", progress_percent)
                        ></div>
                    </div>
                </div>
            </div>

            <div class="px-6 py-4 bg-gray-50 dark:bg-gray-800/50 border-t border-gray-200 dark:border-gray-700">
                <a
                    href=format!("/student/courses/{}/player", course_id)
                    class="block w-full text-center px-4 py-2.5 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 active:bg-blue-800 transition-colors duration-200 shadow-sm"
                >
                    // i18n: t!("student.courses.continue_learning")
                    {if progress_percent == 0 { "Начать обучение" } else { "Продолжить обучение" }}
                </a>
            </div>
        </div>
    }
}

/// Скелетон карточки для состояния загрузки (Suspense fallback).
#[component]
fn SkeletonCard() -> impl IntoView {
    view! {
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 animate-pulse">
            <div class="flex justify-between mb-4">
                <div class="h-5 bg-gray-200 dark:bg-gray-700 rounded w-20"></div>
                <div class="h-5 bg-gray-200 dark:bg-gray-700 rounded w-12"></div>
            </div>
            <div class="h-6 bg-gray-200 dark:bg-gray-700 rounded w-3/4 mb-3"></div>
            <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-full mb-2"></div>
            <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-5/6 mb-6"></div>
            <div class="h-2 bg-gray-200 dark:bg-gray-700 rounded-full w-full mb-4"></div>
            <div class="h-10 bg-gray-200 dark:bg-gray-700 rounded-lg w-full"></div>
        </div>
    }
}

/// Server-функция для получения списка зачислений с прогрессом.
///
/// В реальной реализации здесь будет вызов API-клиента к эндпоинту:
/// `GET /api/v1/courses/me/enrollments` (или аналогичному из Этапа 7/9).
#[server(GetEnrolledCoursesWithProgress, "/api")]
pub async fn fetch_enrolled_courses_with_progress() -> Result<Vec<CourseEnrollment>, ServerFnError> {
    // TODO: Интеграция с реальным API-клиентом или прямой вызов сервиса через контекст Leptos
    // Пример:
    // let ctx = expect_context::<ApiContext>();
    // ctx.client.get_enrolled_courses().await.map_err(ServerFnError::from)

    // Временная заглушка для демонстрации структуры и компиляции
    Ok(vec![])
}