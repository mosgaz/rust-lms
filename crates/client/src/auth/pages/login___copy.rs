// crates/client/src/auth/pages/login.rs
use leptos::prelude::*;
// use crate::ui::Button;
use crate::ui::{Button, ButtonVariant, ButtonSize};

#[component]
pub fn Login() -> impl IntoView {
    view! {
        <div class="text-center">
            <h2 class="text-2xl font-bold text-gray-900 mb-6">Вход в систему</h2>
            <form class="space-y-4 text-left">
                <div>
                    <label class="block text-sm font-medium text-gray-700">Email</label>
                    <input 
                        type="email" 
                        class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" 
                        placeholder="admin@example.com" 
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700">Пароль</label>
                    <input 
                        type="password" 
                        class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500" 
                    />
                </div>
                // <button 
                //     type="button" 
                //     class="w-full py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 transition"
                // >
                //     Войти
                // </button>
				<Button 
                   	href="/admin"
                    variant=ButtonVariant::Default
                    size=ButtonSize::Lg 
                    class="w-full" // tw_merge умно перезапишет базовый 'w-fit' на 'w-full'
                >
                    "Войти"
                </Button>
            </form>
            
            <div class="mt-4 text-sm text-center space-y-2">
                <a href="/register" class="text-blue-600 hover:underline">Нет аккаунта? Зарегистрироваться</a>
                <br />
                <a href="/reset" class="text-gray-500 hover:text-gray-700">Забыли пароль?</a>
            </div>
        </div>
    }
}