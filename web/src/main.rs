#[cfg(feature = "server")]
fn main() {
    dioxus_server::serve(|| async move {
        use dioxus_server::axum::Router;
        use dioxus_server::{DioxusRouterExt as _, FullstackState, ServerFunction};

        let cfg = serve_config();
        web::cleanup_generated_route_cache(public_dir())
            .expect("failed to clear generated route cache");
        let app_router = Router::new().serve_dioxus_application(cfg.clone(), web::App);
        let app_router = with_base_path(app_router, cfg);

        let mut static_routes_router = Router::new();
        for func in ServerFunction::collect() {
            if func.path() == "/api/static_routes" {
                static_routes_router =
                    static_routes_router.route(func.path(), func.method_router());
            }
        }

        Ok(static_routes_router
            .with_state(FullstackState::headless())
            .merge(app_router))
    });
}

#[cfg(all(feature = "web", not(feature = "server")))]
fn main() {
    dioxus::launch(web::App);
}

#[cfg(not(any(feature = "web", feature = "server")))]
fn main() {}

#[cfg(feature = "server")]
fn public_dir() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("DIOXUS_PUBLIC_PATH") {
        return path.into();
    }

    std::env::current_exe()
        .expect("server binary path should be available")
        .parent()
        .expect("server binary should have a parent directory")
        .join("public")
}

#[cfg(feature = "server")]
fn serve_config() -> dioxus_server::ServeConfig {
    dioxus_server::ServeConfig::builder()
        .incremental(
            dioxus_server::IncrementalRendererConfig::new()
                .static_dir(public_dir())
                .clear_cache(false),
        )
        .enable_out_of_order_streaming()
}

#[cfg(feature = "server")]
fn with_base_path(
    app_router: dioxus_server::axum::Router<()>,
    cfg: dioxus_server::ServeConfig,
) -> dioxus_server::axum::Router<()> {
    use dioxus_server::FullstackState;
    use dioxus_server::axum::{
        Router,
        body::Body,
        extract::{Request, State},
        routing::get,
    };

    let Some(base_path) = dioxus::cli_config::base_path() else {
        return app_router;
    };

    let base_path = base_path.trim_matches('/');

    Router::new()
        .nest(&format!("/{base_path}/"), app_router)
        .route(
            &format!("/{base_path}"),
            get(
                |State(state): State<FullstackState>, mut request: Request<Body>| async move {
                    *request.uri_mut() = "/".parse().expect("root route should parse");
                    FullstackState::render_handler(State(state), request).await
                },
            )
            .with_state(FullstackState::new(cfg, web::App)),
        )
}
