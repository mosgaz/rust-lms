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
use leptos::prelude::*;
use leptos_router::{
    path,
    components::{Router, Routes, Route, ParentRoute},
};

// Website
use crate::website::layout::WebsiteLayout;
use crate::website::pages::landing::Landing;

// Auth
use crate::auth::layout::AuthLayout;
use crate::auth::pages::login::Login;
// use crate::auth::pages::register::Register; // Будет добавлено позже
// use crate::auth::pages::reset::Reset;       // Будет добавлено позже

// Shared layouts
use crate::shared::layouts::cpanel::CPanelLayout;

// Admin
use crate::admin::pages::dashboard::AdminDashboard;

// Student
use crate::student::pages::dashboard::StudentDashboard;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound /> }>
                
                // Website
                <ParentRoute path=path!("/") view=WebsiteLayout>
                    <Route path=path!("") view=Landing />
                </ParentRoute>

                // Auth (Чистые URL, макет применяется через обертку)
                <Route path=path!("/login") view=|| view! { <AuthLayout><Login/></AuthLayout> } />
                // <Route path=path!("/register") view=|| view! { <AuthLayout><Register/></AuthLayout> } />
                // <Route path=path!("/reset") view=|| view! { <AuthLayout><Reset/></AuthLayout> } />

                // Admin
                <ParentRoute path=path!("/admin") view=CPanelLayout>
                    <Route path=path!("") view=AdminDashboard />
                </ParentRoute>

                // Student
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
            <h1 class="text-4xl font-bold text-gray-800">404</h1>
            <p class="mt-2 text-gray-600">Страница не найдена</p>
            <a href="/" class="mt-4 text-blue-600 hover:underline">На главную</a>
        </div>
    }
}