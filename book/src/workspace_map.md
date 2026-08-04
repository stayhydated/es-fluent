# Choose crates

Most applications need the `es-fluent` facade, one runtime manager,
and the CLI during development.

| Need | Crate or command |
| --- | --- |
| Typed messages, labels, variants, and choices | `es-fluent` |
| General Rust, CLI, TUI, or desktop runtime | `es-fluent-manager-embedded` |
| Dioxus client or SSR runtime | `es-fluent-manager-dioxus` |
| Bevy ECS and UI runtime | `es-fluent-manager-bevy` |
| Typed locale enum and language labels | `es-fluent-lang` |
| Generate, check, sync, format, and inspect FTL | `cargo es-fluent` from `es-fluent-cli` |
| Rebuild when locale assets change | `es-fluent-build` under `[build-dependencies]` |

A general Rust application can start with:

~~~toml
[dependencies]
es-fluent = "0.18"
es-fluent-manager-embedded = "0.18"
unic-langid = "0.9"
~~~

Install the CLI separately:

~~~sh
cargo install es-fluent-cli --locked
~~~

## Compatible release lines

The framework-specific managers follow their framework version:

| Surface | Release line | Runtime compatibility |
| --- | --- | --- |
| `es-fluent`, CLI, embedded manager, and language enum | `0.18.x` | General Rust |
| `es-fluent-manager-dioxus` | `0.7.x` | Dioxus `0.7.x` |
| `es-fluent-manager-bevy` | `0.19.x` | Bevy `0.19.x` |

## Supporting crates

Application code normally uses the facade and a concrete manager. The following
crates are intended for narrower integration work:

- `es-fluent-derive` and `es-fluent-lang-macro` implement
  macros re-exported by the public facades.
- `es-fluent-manager-core` exposes shared runtime contracts for custom
  manager integrations.
- `es-fluent-manager-macros` exposes the manager module and Bevy text
  macros re-exported by concrete managers.

Continue with [Getting started](getting_started.md), or choose a manager in
[Runtime managers](managers.md).
