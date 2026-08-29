# es-fluent

[![Build status](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Book](https://img.shields.io/badge/docs-book-black)](https://stayhydated.github.io/es-fluent/book/)
[![API docs](https://docs.rs/es-fluent/badge.svg)](https://docs.rs/es-fluent/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent.svg)](https://crates.io/crates/es-fluent)

`es-fluent` provides typed
[Project Fluent](https://projectfluent.org/) localization for Rust. Derive
messages from structs and enums, maintain FTL resources with
`cargo es-fluent`, and resolve them with an embedded, Dioxus, or Bevy
runtime manager. The TypeScript export carries the same Rust-owned message
contract into Project Fluent's JavaScript runtime, with Solid and React
facades plus a universal Expo facade backed by Rust through UniFFI on native.

## Choose a runtime

| Application | Manager |
| --- | --- |
| General Rust, CLI, TUI, or desktop | `es-fluent-manager-embedded` |
| Dioxus client or SSR | `es-fluent-manager-dioxus` |
| Bevy | `es-fluent-manager-bevy` |
| TypeScript, Node, or browser | `@es-fluent/core` |
| Solid 2 or SolidStart | `@es-fluent/solid` |
| React web or React SSR | `@es-fluent/react` |
| Expo on iOS, Android, or web | `@es-fluent/expo` |

Framework managers follow their framework release lines:

| Surface | Compatible line |
| --- | --- |
| Core crates, CLI, embedded manager, and language enum | `0.18.x` |
| Dioxus manager and Dioxus | `0.7.x` |
| Bevy manager and Bevy | `0.19.x` |
| Solid facade and Solid | `2.0.0` RC (`^2.0.0-rc.0`) |
| React web facade and React | `19.2.x` |
| Expo facade and Expo | SDK 57 |

## Quick start

Add the facade and embedded manager:

~~~toml
[dependencies]
es-fluent = "0.18"
es-fluent-manager-embedded = "0.18"
unic-langid = "0.9"

[build-dependencies]
es-fluent-build = "0.18"
~~~

Create `i18n.toml` beside `Cargo.toml`:

~~~toml
fallback_language = "en"
assets_dir = "assets/locales"
~~~

Track locale assets and generate the strict fallback-message catalog:

~~~rust,no_run
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
~~~

Keep localizable types and the manager module reachable from a library target:

~~~rust
// src/lib.rs
pub mod i18n;

use es_fluent::EsFluent;

#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
}
~~~

~~~rust
// src/i18n.rs
es_fluent_manager_embedded::define_i18n_module!();
~~~

Install the CLI, create the fallback locale, and generate FTL:

~~~sh
cargo install es-fluent-cli --locked
mkdir -p assets/locales/en
cargo es-fluent doctor
cargo es-fluent generate
~~~

Follow the [getting-started tutorial](https://stayhydated.github.io/es-fluent/book/getting_started.html)
to edit the fallback message and localize it at runtime.

Configured packages use strict fallback-message validation by default. Set
`missing_message_policy = "fallback-str"` in that package's `i18n.toml` when
normal typed lookup should return the snake_case Rust type, field, or variant
name after locale fallback is exhausted. Strict and fallback-string packages
can coexist in one workspace build; fallible lookup remains `None`.

## TypeScript, Solid, React, and Expo

Generate typed descriptors and package-owned FTL assets from the Rust derive
inventory:

~~~sh
cargo es-fluent export typescript --out web/src/i18n/generated
~~~

`@es-fluent/core` runs entirely in TypeScript on `@fluent/bundle`: Rust supplies
authoring and export, while the application runtime is TypeScript.
`@es-fluent/solid` targets the Solid 2.0 RC line and adds a context provider,
reactive formatting accessor, race-safe locale switching, and revision-checked
SSR snapshots. Request-local locale selection and a shared immutable bundle
cache support concurrent SolidStart requests.

`@es-fluent/react` supplies the equivalent external store, context provider,
and SSR-safe hook for React 19 applications. `@es-fluent/expo` re-exports that
React facade and selects the runtime per platform from one public API: iOS and
Android pass the exported manifest and FTL strings into Rust through generated
UniFFI Swift and Kotlin bindings, while Expo web uses the TypeScript runtime.

See [TypeScript, React, and SolidStart](https://stayhydated.github.io/es-fluent/book/typescript.html)
for web loading and hydration, and [Expo universal facade](https://stayhydated.github.io/es-fluent/book/expo.html)
for the universal Expo workflow and Rust-backed mobile path.

Runnable examples live under [`examples/`](examples):

- [`typescript-example`](examples/typescript-example) uses the framework-neutral browser runtime.
- [`solid-example`](examples/solid-example) targets Solid 2.0 RC.
- [`react-example`](examples/react-example) targets React 19 web.
- [`expo-example`](examples/expo-example) targets Expo on iOS, Android, and web.

The four TypeScript runtime integrations run together on the hosted
[TypeScript demos](https://stayhydated.github.io/es-fluent/typescript-examples/)
page linked from the Demos gallery. Dioxus, Bevy, and GPUI retain dedicated
demo pages. Each TypeScript entry names its framework and runtime package,
including `@es-fluent/expo` for Expo web.

## Documentation

- [User guide](https://stayhydated.github.io/es-fluent/book/)
- [Derive reference](https://stayhydated.github.io/es-fluent/book/deriving_messages.html)
- [Runtime managers](https://stayhydated.github.io/es-fluent/book/managers.html)
- [TypeScript, React, and SolidStart](https://stayhydated.github.io/es-fluent/book/typescript.html)
- [Expo universal facade](https://stayhydated.github.io/es-fluent/book/expo.html)
- [CLI reference](https://stayhydated.github.io/es-fluent/book/cli.html)
- [Rust API documentation](https://docs.rs/es-fluent/)
