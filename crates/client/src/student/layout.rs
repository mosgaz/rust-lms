// crates/client/src/student/layout.rs
use leptos::prelude::*;
use leptos_router::components::Outlet;
use strum::IntoEnumIterator;

use crate::icons::{BookOpen, PanelLeft};
use crate::shared::layouts::cpanel::{render_sub_links, CPanelBreadcrumbs, CPanelMobileSheet, CPanelSidenavHeader, CPanelSidenavFooter};
use crate::student::routes::StudentRoutes;
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
pub fn StudentPanelLayout() -> impl IntoView {
    view! {
        <div class="bg-background">
            <SidenavWrapper attr:style="--sidenav-width:16rem;">
                <Sidenav>
                    <StudentSidenavContent />
                </Sidenav>

                <div class="flex flex-col flex-1 min-h-svh">
                    <header class="flex items-center justify-between px-4 md:px-8 py-3 bg-card border-b">
                        <div class="flex items-center gap-3">
                            // * Desktop-триггер: сворачивает/разворачивает сайдбар
							<div class="hidden md:block">
								<SidenavTrigger>
									<PanelLeft class="size-4" />
								</SidenavTrigger>
							</div>

							// * Mobile-триггер: открывает Sheet (внутри div.md:hidden)
							<CPanelMobileSheet content=|| view! { <StudentSidenavContent /> } />
                            <Separator orientation=SeparatorOrientation::Vertical class="h-4" />
							<CPanelBreadcrumbs
                                root_title="Рабочий стол"
                                root_href="/student"
                                resolve=resolve_student_crumbs
                            />
                        </div>
                        <div class="flex items-center gap-4">
                            <span class="hidden md:inline text-sm text-muted-foreground">
                                "student@rust-lms.local"
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
fn StudentSidenavContent() -> impl IntoView {
    view! {
        <SidenavHeader>
            // <a href="/student" class="flex items-center gap-2 font-medium px-2 py-1">
            //     <div class="flex justify-center items-center rounded-md bg-primary text-primary-foreground size-6">
            //         <GraduationCap class="size-4" />
            //     </div>
            //     "Rust LMS · Student"
            // </a>
			<CPanelSidenavHeader />
        </SidenavHeader>

        <SidenavContent>
            <SidenavGroup>
                <SidenavGroupLabel>"Обучение"</SidenavGroupLabel>
                <SidenavMenu>
                    <SidenavMenuItem>
                        <AccordionItem>
                            <AccordionTrigger open=true class="p-2 peer-checked:bg-accent hover:bg-accent">
                                <AccordionHeader>
                                    <BookOpen />
                                    <AccordionTitle>"Курсы"</AccordionTitle>
                                </AccordionHeader>
                            </AccordionTrigger>
                            <AccordionContent class="p-0">
                                <SidenavMenuSub>
                                    {render_sub_links(
                                        StudentRoutes::iter()
                                            .filter(|r| matches!(r, StudentRoutes::MyCourses | StudentRoutes::Schedule))
                                            .map(|r| (r.to_route(), r.to_title()))
                                            .collect(),
                                    )}
                                </SidenavMenuSub>
                            </AccordionContent>
                        </AccordionItem>
                    </SidenavMenuItem>

                    <SidenavMenuItem>
                        <SidenavLink href=StudentRoutes::Grades.to_route()>
                            <span>{StudentRoutes::Grades.to_title()}</span>
                        </SidenavLink>
                    </SidenavMenuItem>
                </SidenavMenu>
            </SidenavGroup>

            <SidenavGroup>
                <SidenavGroupLabel>"Прочее"</SidenavGroupLabel>
                <SidenavMenu>
					<SidenavMenuItem>
            			<SidenavLink href=StudentRoutes::Profile.to_route()>
							<span>{StudentRoutes::Profile.to_title()}</span>
						</SidenavLink>
					</SidenavMenuItem>
                    <SidenavMenuItem>
                        <SidenavLink href=StudentRoutes::Settings.to_route()>
                            <span>{StudentRoutes::Settings.to_title()}</span>
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
				name="Студент"
				email="student@rust-lms.local"
				initials="СТ"
				profile_href=StudentRoutes::Profile.to_route()
			/>
        </SidenavFooter>
    }
}

/* ========================================================== */
/*                     ✨ BREADCRUMBS RESOLVER ✨             */
/* ========================================================== */

fn resolve_student_crumbs(path: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for route in StudentRoutes::iter() {
        if path.contains(route.as_ref()) {
            out.push((route.to_title().to_string(), None));
        }
    }
    out
}