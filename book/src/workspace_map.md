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
| Track locale assets and compile-check fallback messages | `es-fluent-build` under `[build-dependencies]` |

A general Rust application can start with:

~~~toml
[dependencies]
es-fluent = "0.21"
es-fluent-manager-embedded = "0.21"
unic-langid = "0.9"

[build-dependencies]
es-fluent-build = "0.21"
~~~

Install the CLI separately:

~~~sh
cargo install es-fluent-cli --locked
~~~

Or install a prebuilt binary with
[cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

~~~sh
cargo binstall es-fluent-cli
~~~

## Compatible release lines

The workspace packages use the 0.21 release line. Match each manager to its
framework version:

| Surface | Release line | Runtime compatibility |
| --- | --- | --- |
| `es-fluent`, CLI, embedded manager, and language enum | `0.21.x` | General Rust |
| `es-fluent-manager-dioxus` | `0.21.x` | Dioxus `0.7.x` |
| `es-fluent-manager-bevy` | `0.21.x` | Bevy `0.20.x` |

## Supporting crates

Application code normally uses the facade and a concrete manager. The following
crates are intended for narrower integration work:

- `es-fluent-derive` and `es-fluent-lang-macro` implement
  macros re-exported by the public facades.
- `es-fluent-manager-core` exposes shared runtime contracts for custom
  manager integrations.
- `es-fluent-manager-macros` exposes the manager module and Bevy text
  macros re-exported by concrete managers.

For custom localization tooling, choose the lowest-level API that owns the
operation you need:

| Need | Supporting crate |
| --- | --- |
| Parse and validate package-local `i18n.toml` and resolve asset paths | `es-fluent-toml` |
| Generate, merge, clean, or format FTL outside the CLI | `es-fluent-generate` |
| Consume typed metadata, names, locales, and asset paths without proc-macro code | `es-fluent-shared` |
| Reuse derive parsing and validation in a proc-macro integration | `es-fluent-derive-core` |
| Exchange requests, inventory, results, and filesystem plans with a compatible runner | `es-fluent-runner` |
| Collect derive inventory and plan operations inside a compatible runner | `es-fluent-cli-helpers` |

Use `ResolvedI18nLayout::from_manifest_dir` from `es-fluent-toml` when a tool
needs a validated layout for one package. Its input is the package directory,
not the workspace root unless that root owns the package. See
[Configure a project](configuration.md) for the configuration contract.

When coordinating generated changes across packages, use
`es_fluent_generate::plan_generate` to collect each package's `FileTransaction`
before applying any changes. `generate` plans and applies one package's changes;
it does not coordinate a workspace-wide operation. Prefer the
[CLI](cli.md) when you need its package selection and transaction orchestration.

The runner and helper crates serve CLI integrations rather than application
localization. Keep their request, inventory, and result formats compatible with
the CLI that launches the runner. Applications should continue to use the
facade and a runtime manager.

Continue with [Getting started](getting_started.md), or choose a manager in
[Runtime managers](managers.md).
