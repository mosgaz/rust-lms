// crates/client/src/main.rs
#[cfg(feature = "ssr")]
use leptos::config::get_configuration;
#[cfg(feature = "ssr")]
use leptos::prelude::*;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use rust_lms_client::App;
    use tower_http::services::ServeDir;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options.clone();
    let site_root = leptos_options.site_root.clone();

    let routes = generate_route_list(App);

    let app = Router::new()
        .nest_service("/pkg", ServeDir::new(format!("{}/pkg", site_root)))
        .nest_service("/public", ServeDir::new(&*site_root))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Сервер запущен на http://{}", addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(feature = "ssr")]
fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos_meta::{MetaTags, Stylesheet, provide_meta_context};
    use rust_lms_client::App;

    provide_meta_context();

    view! {
        <!DOCTYPE html>
        <html lang="ru">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <MetaTags/>
                <Stylesheet id="leptos" href="/pkg/rust-lms-client.css"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone()/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {}
