#[cfg(feature = "web")]
fn main() {
    dioxus::launch(web::DevApp);
}

#[cfg(not(feature = "web"))]
fn main() -> anyhow::Result<()> {
    web::run()
}
