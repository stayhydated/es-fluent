#[cfg(not(feature = "web"))]
compile_error!("web must be built with the `web` feature enabled");

#[cfg(not(feature = "web"))]
fn main() {}

#[cfg(feature = "web")]
fn main() {
    stayhydated_site::launch(stayhydated_site::SiteApp::builder().app(web::App).build());
}
