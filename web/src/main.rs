#[cfg(feature = "web")]
fn main() {
    dioxus::launch(web::App);
}

#[cfg(not(feature = "web"))]
fn main() -> anyhow::Result<()> {
    web::run()
}
