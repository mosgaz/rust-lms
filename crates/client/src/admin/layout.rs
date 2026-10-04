// crates/client/src/admin/layout.rs
use leptos::prelude::*;
use leptos_router::components::Outlet;
use strum::IntoEnumIterator;

use crate::admin::routes::AdminRoutes;
use crate::icons::{ShieldCheck, PanelLeft};
use crate::shared::layouts::cpanel::{render_sub_links, CPanelBreadcrumbs, CPanelMobileSheet, CPanelSidenavHeader, CPanelSidenavFooter};
use crate::ui::{
    AccordionContent, AccordionHeader, AccordionItem, AccordionTitle, AccordionTrigger,
    Sidenav, SidenavContent, SidenavFooter, SidenavGroup, SidenavGroupLabel,
    SidenavHeader, SidenavLink, SidenavMenu, SidenavMenuItem, SidenavMenuSub, SidenavWrapper,
};
use crate::ui::SidenavTrigger;
use crate::ui::{Separator, SeparatorOrientation};

/* ========================================================== */
/*                     ✨ LAYOUT ✨                           */
/* ========================================================== */

#[component]
pub fn AdminPanelLayout() -> impl IntoView {
    view! {
        <div class="bg-background">
            <SidenavWrapper attr:style="--sidenav-width:16rem;">
                <Sidenav>
                    <AdminSidenavContent />
                </Sidenav>

                <div class="flex flex-col flex-1 min-h-svh">
                    <header class="flex items-center justify-between px-4 md:px-8 py-3 bg-card border-b">
                        <div class="flex items-center gap-3">
							// * Desktop: сворачивает/разворачивает Sidenav (Ctrl+B)
							<div class="hidden md:block">
								<SidenavTrigger>
									<PanelLeft class="size-4" />
								</SidenavTrigger>
							</div>
							// * Mobile: открывает Sheet
                            <CPanelMobileSheet content=|| view! { <AdminSidenavContent /> } />
                            <Separator orientation=SeparatorOrientation::Vertical class="h-4" />
							<CPanelBreadcrumbs
                                root_title="Рабочий стол"
                                root_href="/admin"
                                resolve=resolve_admin_crumbs
                            />
                        </div>
                        <div class="flex items-center gap-4">
                            <span class="hidden md:inline text-sm text-muted-foreground">
                                "admin@rust-lms.local"
                            </span>
                            <div class="w-8 h-8 rounded-full bg-primary"></div>
                        </div>
                    </header>

                    <main class="flex-1 p-4 md:p-8 overflow-y-auto">
                        <Outlet />
                    </main>
                </div>
            </SidenavWrapper>
        </div>
    }
}

/* ========================================================== */
/*                     ✨ SIDENAV ✨                          */
/* ========================================================== */

#[component]
fn AdminSidenavContent() -> impl IntoView {
    view! {
        <SidenavHeader>
            // <a href="/admin" class="flex items-center gap-2 font-medium px-2 py-1">
            //     <div class="flex justify-center items-center rounded-md bg-primary text-primary-foreground size-6">
            //         <GraduationCap class="size-4" />
            //     </div>
            //     "Rust LMS · Admin"
            // </a>
			<CPanelSidenavHeader />
        </SidenavHeader>

        <SidenavContent>
            <SidenavGroup>
                <SidenavGroupLabel>"Управление"</SidenavGroupLabel>
                <SidenavMenu>
                    <SidenavMenuItem>
                        <AccordionItem>
                            <AccordionTrigger class="p-2 peer-checked:bg-accent hover:bg-accent">
                                <AccordionHeader>
                                    <ShieldCheck />
                                    <AccordionTitle>"Контент"</AccordionTitle>
                                </AccordionHeader>
                            </AccordionTrigger>
                            <AccordionContent class="p-0">
                                <SidenavMenuSub>
                                    {render_sub_links(
                                        AdminRoutes::iter()
                                            .filter(|r| matches!(r, AdminRoutes::Courses | AdminRoutes::Tests))
                                            .map(|r| (r.to_route(), r.to_title()))
                                            .collect(),
                                    )}
                                </SidenavMenuSub>
                            </AccordionContent>
                        </AccordionItem>
                    </SidenavMenuItem>

                    <SidenavMenuItem>
                        <SidenavLink href=AdminRoutes::Users.to_route()>
                            <span>{AdminRoutes::Users.to_title()}</span>
                        </SidenavLink>
                    </SidenavMenuItem>
                </SidenavMenu>
            </SidenavGroup>

            <SidenavGroup>
                <SidenavGroupLabel>"Прочее"</SidenavGroupLabel>
                <SidenavMenu>
					<SidenavMenuItem>
						<SidenavLink href=AdminRoutes::Profile.to_route()>
							<span>{AdminRoutes::Profile.to_title()}</span>
						</SidenavLink>
					</SidenavMenuItem>
                    <SidenavMenuItem>
                        <SidenavLink href=AdminRoutes::Settings.to_route()>
                            <span>{AdminRoutes::Settings.to_title()}</span>
                        </SidenavLink>
                    </SidenavMenuItem>
                </SidenavMenu>
            </SidenavGroup>
        </SidenavContent>

        <SidenavFooter>
            // <a href="/" class="text-sm text-muted-foreground hover:text-foreground px-2">
            //     "← На сайт"
            // </a>
			<CPanelSidenavFooter
				name="Администратор"
				email="admin@rust-lms.local"
				initials="АД"
				profile_href=AdminRoutes::Profile.to_route()
			/>
        </SidenavFooter>
    }
}

/* ========================================================== */
/*                     ✨ BREADCRUMBS RESOLVER ✨             */
/* ========================================================== */

fn resolve_admin_crumbs(path: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for route in AdminRoutes::iter() {
        if path.contains(route.as_ref()) {
            out.push((route.to_title().to_string(), None));
        }
    }
    out
}