// crates/ui/src/components/faq_transition.rs
use leptos::prelude::*;
use crate::clx;
use tw_merge::*;

mod components {
    use super::*;
    clx! {Faq, div, "flex flex-col gap-3 w-full max-w-screen-md"}
    clx! {FaqTitle, span, "text-lg text-primary"}
    clx! {FaqDescription, p, "pr-6 mb-2 text-muted-foreground"}
    clx! {FaqSection, div, "w-full rounded bg-accent/30 hover:bg-accent flex flex-col"}

    clx! {FaqContentRoot, div, "grid overflow-hidden mt-2 transition-all duration-500 grid-rows-[0fr] peer-checked:grid-rows-[1fr]"}
}

pub use components::*;

/* ========================================================== */
/*                     ✨ FUNCTIONS ✨                        */
/* ========================================================== */

#[component]
pub fn FaqContent(children: Children) -> impl IntoView {
    view! {
        <FaqContentRoot>
            <div class="px-4 min-h-[0]">{children()}</div>
        </FaqContentRoot>
    }
}

#[component]
pub fn FaqLabel(
    #[prop(into, optional)] class: String,
    #[prop(into)] for_attr: &'static str,
    children: Children,
) -> impl IntoView {
    let merged_class = tw_merge!("flex justify-between items-center py-2 px-4 mt-2 w-full cursor-pointer", class);

    view! {
        <label class=merged_class for=for_attr>
            {children()}
        </label>
    }
}

#[component]
pub fn FaqInput(#[prop(into)] id: &'static str) -> impl IntoView {
    view! { <input id=id type="checkbox" class="ml-auto peer" /> } // sr-only
}
