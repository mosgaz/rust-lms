// crates/client/src/auth/layout.rs
// TODO: https://leptos.rust-ui.com/blocks/login
// TODO: https://leptos.rust-ui.com/view/login02
// TODO: Use crates/ui/layouts/split Layout
use leptos::prelude::*;
use crate::icons::GraduationCap;

#[component]
pub fn AuthLayout(children: Children) -> impl IntoView {
    view! {
        <div class="grid lg:grid-cols-2 min-h-svh">
            <div class="flex flex-col gap-4 p-6 md:p-10">
                <div class="flex gap-2 justify-center md:justify-start">
                    <a href="/" class="flex gap-2 items-center font-medium">
                        <div class="flex justify-center items-center rounded-md bg-primary text-primary-foreground size-6">
                            <GraduationCap class="size-4" />
                        </div>
                        Rust Lms
                    </a>
                </div>
                <div class="flex flex-1 justify-center items-center">
                    <div class="w-full max-w-xs">
                        {children()}
                    </div>
                </div>
            </div>
            <div class="hidden relative lg:block bg-muted">
                <img
                    src="/images/placeholder.svg"
                    alt="Image"
                    class="object-cover absolute inset-0 w-full h-full dark:brightness-[0.2] dark:grayscale"
                />
            </div>
        </div>
    }
}