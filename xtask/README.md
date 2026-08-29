# xtask

Repository-maintenance commands for generated documentation, site assets, demos,
and crates.io release planning. Application users should use the published
crates and [user guide](https://stayhydated.github.io/es-fluent/book/).

Run commands from the workspace root:

| Command | Purpose |
| --- | --- |
| `cargo xtask build book` | Build mdBook output for the site. |
| `cargo xtask build llms-txt` | Generate LLM-oriented book exports. |
| `cargo xtask build bevy-demo` | Build the hosted Bevy demo assets. |
| `cargo xtask build gpui-demo` | Build the hosted GPUI demo assets; requires nightly Rust. |
| `cargo xtask build typescript-demos` | Build the hosted TypeScript, Solid 2, React, and Expo web demo assets. |
| `cargo xtask build web` | Build the release Dioxus site. |
| `cargo xtask release plan` | Print crates.io publication order. |
| `cargo xtask release publish` | Print publish commands in release order. |
| `cargo xtask release publish --execute --skip-existing` | Publish while skipping versions already present. |

Use `cargo xtask <COMMAND> --help` for release resume, dirty-worktree,
and dev-dependency options.
