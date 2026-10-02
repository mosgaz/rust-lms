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
                <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"/>
                
				<MetaTags/>
                <Stylesheet id="leptos" href="/pkg/rust-lms-client.css"/>
				
				// Favicon
				<link rel="icon" type="image/svg+xml" href="/images/icons/favicon.svg?v=20261002" />
				<link rel="icon" type="image/png" sizes="96x96" href="/images/icons/icon-96x96.png?v=20261002" />
				<link rel="shortcut icon" href="/favicon.ico?v=20261002" />

				// PWA
				<link rel="manifest" href="/manifest.json" />
				<meta name="theme-color" content="#000000" />
                
				// Apple
				<meta name="mobile-web-app-capable" content="yes" />
				<meta name="apple-mobile-web-app-capable" content="yes" />
				// <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
				<meta name="apple-mobile-web-app-status-bar-style" content="default" />
				<meta name="apple-mobile-web-app-title" content="Rust LMS" />
				<link rel="apple-touch-icon" sizes="180x180" href="/images/icons/icon-180x180.png" />

				// iOS Splash Screens (portrait)
				<link rel="apple-touch-startup-image" href="/splash/splash-2048x2732.png" media="screen and (device-width: 1024px) and (device-height: 1366px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-1668x2224.png" media="screen and (device-width: 834px) and (device-height: 1112px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-1536x2048.png" media="screen and (device-width: 768px) and (device-height: 1024px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-1280x1920.png" media="screen and (device-width: 640px) and (device-height: 960px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-1242x2208.png" media="screen and (device-width: 414px) and (device-height: 736px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-1125x2436.png" media="screen and (device-width: 375px) and (device-height: 812px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-960x1280.png" media="screen and (device-width: 480px) and (device-height: 640px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-750x1334.png" media="screen and (device-width: 375px) and (device-height: 667px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-720x960.png" media="screen and (device-width: 360px) and (device-height: 480px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-480x640.png" media="screen and (device-width: 320px) and (device-height: 480px) and (-webkit-device-pixel-ratio: 2) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-320x470.png" media="screen and (device-width: 320px) and (device-height: 470px) and (-webkit-device-pixel-ratio: 1) and (orientation: portrait)" />
				<link rel="apple-touch-startup-image" href="/splash/splash-320x426.png" media="screen and (device-width: 320px) and (device-height: 426px) and (-webkit-device-pixel-ratio: 1) and (orientation: portrait)" />

				// iOS Splash Screens (portrait / dark theme)
				// <link rel="apple-touch-startup-image" href="/splash/dark/splash-1125x2436.png" media="screen and (prefers-color-scheme: dark) and (device-width: 375px) and (device-height: 812px) and (-webkit-device-pixel-ratio: 3) and (orientation: portrait)" />
				
				// iOS Splash Screens (landscape)
				// <link rel="apple-touch-startup-image" href="/splash/splash-2732x2048.png" media="screen and (device-width: 1024px) and (device-height: 1366px) and (-webkit-device-pixel-ratio: 2) and (orientation: landscape)" />

                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone()/>
            
			</head>
            
			<body>
                
				<App/>
				
				// Скрипт регистрации Service Worker (выполняется только в браузере)
                // Передаем JS-код как сырую строку вовнутрь тега script, чтобы Rust его не компилировал
                <script>
                    "if ('serviceWorker' in navigator) {
                        window.addEventListener('load', () => {
                            navigator.serviceWorker.register('/sw.js')
                                .then(reg => console.log('[PWA] Service Worker registered:', reg.scope))
                                .catch(err => console.error('[PWA] Service Worker registration failed:', err));
                        });
                    }"
                </script>
            
			</body>
        
		</html>
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {}
