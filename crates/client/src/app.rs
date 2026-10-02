// use crate::ui::Button;
// use leptos::prelude::*;

// #[component]
// pub fn App() -> impl IntoView {
//     view! {
//         // Tailwind-классы для центрирования и проверки работы стилей
//         <main class="flex flex-col items-center justify-center min-h-screen bg-gray-50 p-4 gap-6">
//             <h1 class="text-3xl font-bold text-gray-800">
//                 Проверка стилей и компонентов
//             </h1>

//             <Button>
//                 "Тестовая кнопка"
//             </Button>
//         </main>
//     }
// }
// crates/client/src/app.rs
// crates/client/src/app.rs
// crates/client/src/app.rs
// crates/client/src/app.rs
use leptos::prelude::*;
use leptos_router::{
    path,
    components::{Router, Routes, Route, ParentRoute},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

use crate::shared::state::AppQueueState;
use crate::student::services::offline_queue::STUDENT_QUEUE;
use crate::admin::services::offline_queue::ADMIN_QUEUE;

// Website
use crate::website::layout::WebsiteLayout;
use crate::website::pages::landing::Landing;

// Auth
use crate::auth::layout::AuthLayout;
use crate::auth::pages::login::Login;

// Shared layouts
use crate::shared::layouts::cpanel::CPanelLayout;

// Admin
use crate::admin::pages::dashboard::AdminDashboard;

// Student
use crate::student::pages::dashboard::StudentDashboard;

#[cfg(target_arch = "wasm32")]
fn is_online_global() -> bool {
    window().map(|w| w.navigator().on_line()).unwrap_or(true)
}
#[cfg(not(target_arch = "wasm32"))]
fn is_online_global() -> bool {
    true
}

#[component]
pub fn App() -> impl IntoView {
    // 1. Создаем глобальные сигналы для всего приложения
    let is_online_signal = create_rw_signal(is_online_global());
    let student_queue_size = create_rw_signal(STUDENT_QUEUE.with(|q| q.len()));
    let admin_queue_size = create_rw_signal(ADMIN_QUEUE.with(|q| q.len()));

    // 2. Помещаем их в типобезопасную структуру и предоставляем контекст
    provide_context(AppQueueState {
        is_online: is_online_signal,
        student_queue_size,
        admin_queue_size,
    });

    // 3. Настраиваем слушатели сети ОДИН раз
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = window() {
        // Слушатель для Student Queue
        let student_online_closure = Closure::wrap(Box::new({
            let student_queue_size = student_queue_size;
            move || {
                is_online_signal.set(true);
                leptos::logging::log!("[App] Network is back online!");
                
                let queue = STUDENT_QUEUE.with(|q| q.clone());
                let write_sig = student_queue_size.write_only();
                spawn_local(async move {
                    queue.process_queue(write_sig).await;
                });
            }
        }) as Box<dyn FnMut()>);

        // Слушатель для Admin Queue
        let admin_online_closure = Closure::wrap(Box::new({
            let admin_queue_size = admin_queue_size;
            move || {
                let queue = ADMIN_QUEUE.with(|q| q.clone());
                let write_sig = admin_queue_size.write_only();
                spawn_local(async move {
                    queue.process_queue(write_sig).await;
                });
            }
        }) as Box<dyn FnMut()>);

        let offline_closure = Closure::wrap(Box::new(move || {
            is_online_signal.set(false);
            leptos::logging::log!("[App] Network is offline.");
        }) as Box<dyn FnMut()>);

        let _ = window.add_event_listener_with_callback("online", student_online_closure.as_ref().unchecked_ref());
        let _ = window.add_event_listener_with_callback("online", admin_online_closure.as_ref().unchecked_ref());
        let _ = window.add_event_listener_with_callback("offline", offline_closure.as_ref().unchecked_ref());
        
        student_online_closure.forget();
        admin_online_closure.forget();
        offline_closure.forget();
    }

    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound /> }>
                
                <ParentRoute path=path!("/") view=WebsiteLayout>
                    <Route path=path!("") view=Landing />
                </ParentRoute>

                <Route path=path!("/login") view=|| view! { 
                    <crate::auth::layout::AuthLayout><Login/></crate::auth::layout::AuthLayout> 
                } />

                <ParentRoute path=path!("/admin") view=CPanelLayout>
                    <Route path=path!("") view=AdminDashboard />
                </ParentRoute>

                <ParentRoute path=path!("/student") view=CPanelLayout>
                    <Route path=path!("") view=StudentDashboard />
                </ParentRoute>
            </Routes>
        </Router>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center min-h-screen bg-gray-50">
            <h1 class="text-4xl font-bold text-gray-800">"404"</h1>
            <p class="mt-2 text-gray-600">"Страница не найдена"</p>
            <a href="/" class="mt-4 text-blue-600 hover:underline">"На главную"</a>
        </div>
    }
}