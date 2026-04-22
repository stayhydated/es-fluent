pub(crate) const SITE_URL: &str = "https://stayhydated.github.io/es-fluent/";
pub(crate) const README_URL: &str =
    "https://github.com/stayhydated/es-fluent/blob/master/README.md";
pub(crate) const CRATES_URL: &str = "https://github.com/stayhydated/es-fluent/tree/master/crates";
pub(crate) const DIOXUS_EXAMPLE_URL: &str =
    "https://github.com/stayhydated/es-fluent/tree/master/examples/dioxus-example";
#[cfg(feature = "web")]
pub(crate) const DEV_SITE_STYLE: &str = include_str!("../../assets/site.css");
pub(crate) const INSTALL_SNIPPET: &str = r#"[dependencies]
es-fluent = { version = "*", features = ["derive"] }
unic-langid = "*"

# Pick one runtime manager:
es-fluent-manager-embedded = "*"
es-fluent-manager-bevy = "*"
es-fluent-manager-dioxus = { version = "*", features = ["web"] }"#;
pub(crate) const BEVY_BOOTSTRAP: &str = r#"const root = document.getElementById("bevy-loader");
if (!root) {
  throw new Error("Missing Bevy loader root");
}

const setState = (state) => {
  root.dataset.state = state;
};

(async () => {
  try {
    const moduleUrl = new URL("bevy-example.js", window.location.href);
    const wasmModule = await import(moduleUrl.href);
    if (typeof wasmModule.default !== "function") {
      throw new Error("Missing default init export");
    }
    await wasmModule.default();
    setState("ready");
  } catch (error) {
    console.error(error);
    setState("error");
  }
})();"#;
