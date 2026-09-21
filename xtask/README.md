# xtask

Repository-maintenance commands for generated documentation, site assets,
and demos.

Run commands from the workspace root:

| Command | Purpose |
| --- | --- |
| `cargo xtask build book` | Build mdBook output for the site. |
| `cargo xtask build llms-txt` | Generate LLM-oriented book exports. |
| `cargo xtask build bevy-demo` | Build the hosted Bevy demo assets. |
| `cargo xtask build gpui-demo` | Build the hosted GPUI demo assets; requires nightly Rust. |
| `cargo xtask build web` | Build the release Dioxus site. |
