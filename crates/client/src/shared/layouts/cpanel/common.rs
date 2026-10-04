// crates/client/src/shared/layouts/cpanel/common.rs
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::icons::{ChevronsUpDown, GraduationCap, PanelLeft};
use crate::ui::{
    DropdownMenu, DropdownMenuAction, DropdownMenuAlign, DropdownMenuContent,
    DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel, DropdownMenuLink,
    DropdownMenuPosition, DropdownMenuTrigger,
};
use crate::ui::Separator;
use crate::ui::{
    Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList,
    BreadcrumbPage, BreadcrumbSeparator,
    ButtonSize, ButtonVariant,
    Sheet, SheetContent, SheetContext, SheetDirection, SheetTrigger,
    SidenavLink,
};

/* ========================================================== */
/*                     ✨ BREADCRUMBS ✨                      */
/* ========================================================== */

#[derive(Clone, PartialEq)]
pub struct Crumb {
    pub title: String,
    pub href: Option<String>,
}

/// Универсальные breadcrumbs для панелей.
/// `root_title` / `root_href` — первый «корень» (например, «Админка» или «Кабинет студента»).
/// `resolve` — чистая функция, которая по текущему пути возвращает список
/// промежуточных crumbs (без корня). Последний элемент рендерится как `BreadcrumbPage`.
#[component]
pub fn CPanelBreadcrumbs(
    root_title: &'static str,
    root_href: &'static str,
    resolve: fn(&str) -> Vec<(String, Option<String>)>,
) -> impl IntoView {
    let location = use_location();

    let crumbs = move || -> Vec<Crumb> {
        let path = location.pathname.get();
        let mut out = vec![Crumb {
            title: root_title.to_string(),
            href: Some(root_href.to_string()),
        }];
        for (title, href) in resolve(&path) {
            out.push(Crumb { title, href });
        }
        out
    };

    view! {
        <Breadcrumb>
            <BreadcrumbList>
                {move || {
                    let items = crumbs();
                    let last = items.len().saturating_sub(1);
                    items
                        .into_iter()
                        .enumerate()
                        .flat_map(|(i, c)| {
                            let is_last = i == last;
                            let node = if is_last {
                                view! { <BreadcrumbPage>{c.title.clone()}</BreadcrumbPage> }.into_any()
                            } else {
                                view! {
                                    <BreadcrumbLink attr:href=c.href.clone().unwrap_or_default()>
                                        {c.title.clone()}
                                    </BreadcrumbLink>
                                }
                                    .into_any()
                            };
                            if is_last {
                                vec![view! { <BreadcrumbItem>{node}</BreadcrumbItem> }.into_any()]
                            } else {
                                vec![
                                    view! { <BreadcrumbItem>{node}</BreadcrumbItem> }.into_any(),
                                    view! { <BreadcrumbSeparator /> }.into_any(),
                                ]
                            }
                        })
                        .collect_view()
                }}
            </BreadcrumbList>
        </Breadcrumb>
    }
}

/* ========================================================== */
/*                     ✨ MOBILE SHEET ✨                     */
/* ========================================================== */

/// Мобильный sheet, оборачивающий переданный sidenav-контент.
/// `content` — функция-фабрика, возвращающая view с содержимым сайдбара
/// (используется и в desktop-Sidenav, и в mobile Sheet).
#[component]
pub fn CPanelMobileSheet<F, V>(content: F) -> impl IntoView
where
    F: Fn() -> V + Send + 'static,
    V: IntoView + 'static,
{
    view! {
        <Sheet>
            <div class="md:hidden">
                <SheetTrigger class="size-7" variant=ButtonVariant::Ghost size=ButtonSize::Icon>
                    <PanelLeft class="size-4" />
                    <span class="hidden">"Toggle Sidenav"</span>
                </SheetTrigger>
            </div>
            <SheetContent
                direction=SheetDirection::Left
                class="p-0 w-[18rem] bg-sidenav text-sidenav-foreground"
                show_close_button=false
            >
                <div class="flex flex-col h-full">{content()}</div>
            </SheetContent>
        </Sheet>
    }
}

/* ========================================================== */
/*                 ✨ LINK LIST INSIDE SUBMENU ✨             */
/* ========================================================== */

/// Рендерит список ссылок внутри `SidenavMenuSub`.
/// Когда внутри мобильного Sheet — оборачивает каждую ссылку в
/// `data-sheet-close`, чтобы sheet закрывался при клике.
pub fn render_sub_links(links: Vec<(String, &'static str)>) -> impl IntoView {
    let sheet_ctx = use_context::<SheetContext>();
    let target_id = sheet_ctx.as_ref().map(|c| c.target_id.clone());

    links
        .into_iter()
        .map(|(href, title)| {
            if let Some(ref id) = target_id {
                view! {
                    <div data-sheet-close=id.clone()>
                        <SidenavLink href=href>{title}</SidenavLink>
                    </div>
                }
                .into_any()
            } else {
                view! { <SidenavLink href=href>{title}</SidenavLink> }.into_any()
            }
        })
        .collect_view()
}

/// Временный селектор панели в шапке сайдбара.
/// Переключает `/admin` ↔ `/student`. В проде вырезать.
#[component]
pub fn CPanelSidenavHeader() -> impl IntoView {
    let location = use_location();

    // Определяем активную панель по URL
    let current = Memo::new(move |_| {
        let path = location.pathname.get();
        if path.starts_with("/student") { "student" } else { "admin" }
    });

    let items: [(&'static str, &'static str, &'static str); 2] = [
        ("admin", "/admin", "Администратор"),
        ("student", "/student", "Студент"),
    ];

    view! {
        <DropdownMenu align=DropdownMenuAlign::Start>
            <DropdownMenuTrigger class="flex justify-between px-2 w-full h-12 bg-transparent border-0">
                <div class="flex gap-2 items-center">
                    <div class="flex justify-center items-center rounded-lg bg-primary text-primary-foreground aspect-square size-8">
                        <GraduationCap />
                    </div>
                    <div class="grid flex-1 text-sm leading-tight text-left">
                        <span class="font-medium">"Rust LMS"</span>
                        <span class="text-xs text-muted-foreground">
                            {move || match current.get() {
                                "student" => "Кабинет студента",
                                _ => "Кабинет администратора",
                            }}
                        </span>
                    </div>
                </div>
                <ChevronsUpDown />
            </DropdownMenuTrigger>

            <DropdownMenuContent class="w-[200px]">
                <DropdownMenuGroup>
                    {items
                        .into_iter()
                        .map(|(key, href, label)| {
                            let is_active = move || current.get() == key;
                            view! {
                                <DropdownMenuItem attr:data-active=move || is_active().to_string()>
                                    <DropdownMenuAction href=href>
                                        {label}
                                    </DropdownMenuAction>
                                </DropdownMenuItem>
                            }
                        })
                        .collect_view()}
                </DropdownMenuGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    }
}

/// Юзер-меню в футере сайдбара.
/// Показывает аватар + имя + email, при клике — меню вверх.
#[component]
pub fn CPanelSidenavFooter(
    name: &'static str,       // "Админ" / "Студент"
    email: &'static str,      // "admin@rust-lms.local" / "student@rust-lms.local"
    initials: &'static str,   // "АД" / "СТ"
    profile_href: String,     // "/admin/profile" / "/student/profile"
) -> impl IntoView {
    view! {
        <DropdownMenu align=DropdownMenuAlign::EndOuter>
            <DropdownMenuTrigger class="flex justify-between px-2 w-full h-12 bg-transparent border-0">
                <div class="flex gap-2 items-center">
                    <span data-name="avatar" class="flex overflow-hidden relative rounded-lg size-8 shrink-0">
                        <span
                            data-name="avatar-fallback"
                            class="flex justify-center items-center rounded-full bg-secondary size-full"
                        >
                            {initials}
                        </span>
                    </span>

                    <div class="grid flex-1 text-sm leading-tight text-left">
                        <span class="font-medium truncate">{name}</span>
                        <span class="text-xs truncate">{email}</span>
                    </div>
                </div>

                <ChevronsUpDown />
            </DropdownMenuTrigger>

            <DropdownMenuContent class="w-[220px]" position=DropdownMenuPosition::Top>
                <DropdownMenuLabel>"Меню"</DropdownMenuLabel>

                <DropdownMenuGroup>
                    <DropdownMenuItem>
                        <DropdownMenuLink attr:href=profile_href.clone()>
                            "Профиль"
                        </DropdownMenuLink>
                    </DropdownMenuItem>
                    <DropdownMenuItem>
                        <DropdownMenuLink attr:href="/settings">
                            "Настройки"
                        </DropdownMenuLink>
                    </DropdownMenuItem>
                </DropdownMenuGroup>

                <Separator class="my-1" />

                <DropdownMenuGroup>
                    <DropdownMenuItem>
                        <DropdownMenuLink attr:href="/">
                            "На сайт"
                        </DropdownMenuLink>
                    </DropdownMenuItem>
                    <DropdownMenuItem>
                        <DropdownMenuAction>"Выйти"</DropdownMenuAction>
                    </DropdownMenuItem>
                </DropdownMenuGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    }
}