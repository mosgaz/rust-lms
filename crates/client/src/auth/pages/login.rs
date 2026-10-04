// crates/client/src/auth/pages/login.rs
use crate::icons::{
	Eye, 
	EyeOff, 
	// Github,
};
use leptos::prelude::*;

use crate::ui::{Button, ButtonVariant, Input, Label};

/*
 * title: Split Login with GitHub
 */

#[component]
pub fn Login() -> impl IntoView {
    let show_password = RwSignal::new(false);
    let password_input_ref = NodeRef::<leptos::html::Input>::new();

    let toggle_password_visibility = move |_| {
        show_password.update(|value| *value = !*value);
        if let Some(input) = password_input_ref.get() {
            input.set_type(if show_password.get_untracked() { "text" } else { "password" });
        }
    };

    view! {
        <form class="flex flex-col gap-6">
            <div class="flex flex-col gap-2 items-center text-center">
                <h1 class="text-2xl font-bold">"Login to your account"</h1>
                <p class="text-sm text-muted-foreground text-balance">
                    "Enter your email below to login to your account"
                </p>
            </div>
            <div class="grid gap-6">
                <div class="grid gap-3">
                    <Label html_for="email">"Email"</Label>
                    <Input
                        attr:r#type="email"
                        attr:id="email"
                        autocomplete="username"
                        attr:placeholder="m@example.com"
                        attr:required=true
                    />
                </div>
                <div class="grid gap-3">
                    <div class="flex items-center">
                        <Label html_for="password">"Password"</Label>
                        <a href="#" class="ml-auto text-sm hover:underline underline-offset-4">
                            "Forgot your password?"
                        </a>
                    </div>
                    <div class="relative">
                        <Input
                            node_ref=password_input_ref
                            attr:r#type="password"
                            attr:id="password"
                            autocomplete="current-password"
                            minlength=8
                            attr:required=true
                            class="pr-10"
                        />
                        <button
                            type="button"
                            class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                            attr:aria-label=move || {
                                if show_password.get() { "Hide password" } else { "Show password" }
                            }
                            on:click=toggle_password_visibility
                        >
                            {move || {
                                if show_password.get() {
                                    view! {
                                        <>
                                            <EyeOff class="size-4" />
                                            <span class="sr-only">"Hide password"</span>
                                        </>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <>
                                            <Eye class="size-4" />
                                            <span class="sr-only">"Show password"</span>
                                        </>
                                    }
                                        .into_any()
                                }
                            }}
                        </button>
                    </div>
                </div>

				// TODO: Submit form data
                // <Button class="w-full" attr:r#type="submit">
                //     "Login"
                // </Button>
				
				<Button 
                   	href="/admin"
                    variant=ButtonVariant::Default
                    class="w-full" // tw_merge умно перезапишет базовый 'w-fit' на 'w-full'
                >
                    "Войти"
                </Button>

				// TODO: Social login
                // <div class="relative text-sm text-center after:border-border after:absolute after:inset-0 after:top-1/2 after:z-0 after:flex after:items-center after:border-t">
                //     <span class="relative z-10 px-2 bg-background text-muted-foreground">
                //         "Or continue with"
                //     </span>
                // </div>
                // <Button variant=ButtonVariant::Outline class="w-full">
                //     <Github />
                //     <span>"Login with GitHub"</span>
                // </Button>
            </div>
            <div class="text-sm text-center">
                "Don't have an account? " <a href="#" class="underline underline-offset-4">
                    "Sign up"
                </a>
            </div>
        </form>
    }
}