#[cfg(not(any(feature = "web", feature = "server")))]
compile_error!("web must be built with either the `web` or `server` feature enabled");

#[cfg(not(any(feature = "web", feature = "server")))]
fn main() {}

#[cfg(any(feature = "web", feature = "server"))]
fn main() {
    stayhydated_site::launch(
        stayhydated_site::SiteApp::builder()
            .app(web::App)
            .route_cache(web::route_cache())
            .build(),
    );
}
