#[cfg(feature = "ssr")]
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode, Uri},
    response::IntoResponse,
    Router,
};

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::config::LeptosOptions;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::net::SocketAddr;

    use fund::web::app::{shell, App};
    use fund::web::state;

    let db_path = std::env::var("FUND_DB").unwrap_or_else(|_| "fund.db".to_string());
    state::init(&db_path)
        .await
        .expect("failed to open database");

    let addr: SocketAddr = std::env::var("FUND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()
        .expect("invalid FUND_ADDR");

    let leptos_options = LeptosOptions::builder()
        .output_name("fund")
        .site_pkg_dir("pkg")
        .site_addr(addr)
        .env("DEV")
        .build();

    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let options = leptos_options.clone();
            move || shell(options.clone())
        })
        .fallback({
            let options = leptos_options.clone();
            move |uri: Uri| {
                let options = options.clone();
                async move {
                    let path = uri.path().to_string();
                    match serve_asset(&path) {
                        Some(res) => res,
                        None => {
                            let ctx_options = options.clone();
                            let shell_options = options.clone();
                            let handler = leptos_axum::render_app_to_stream_in_order_with_context(
                                move || {
                                    leptos::context::provide_context(ctx_options.clone());
                                },
                                move || shell(shell_options.clone()),
                            );
                            let req = Request::builder()
                                .uri(uri)
                                .body(Body::empty())
                                .expect("build request");
                            handler(req).await
                        }
                    }
                }
            }
        })
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

#[cfg(feature = "ssr")]
#[allow(dead_code)]
const FUND_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pkg/fund.js"));
#[cfg(feature = "ssr")]
#[allow(dead_code)]
const FUND_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pkg/fund_bg.wasm"));
#[cfg(feature = "ssr")]
#[allow(dead_code)]
const STYLES_CSS: &[u8] = include_bytes!("web/styles.css");

#[cfg(feature = "ssr")]
fn serve_asset(path: &str) -> Option<Response<Body>> {
    let (bytes, content_type) = match path {
        "/pkg/fund.js" => (FUND_JS, "text/javascript"),
        "/pkg/fund_bg.wasm" => (FUND_WASM, "application/wasm"),
        "/pkg/styles.css" => (STYLES_CSS, "text/css"),
        _ => return None,
    };
    Some(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, content_type)],
            bytes.to_vec(),
        )
            .into_response(),
    )
}

#[cfg(not(feature = "ssr"))]
fn main() {}
