# Choose a runtime manager

A runtime manager loads FTL resources, selects a locale, and resolves typed
messages through an explicit application context. Choose one manager for the
application runtime.

| Manager | Choose it for | Continue |
| --- | --- | --- |
| `es-fluent-manager-embedded` | CLIs, TUIs, desktop apps, services, and general Rust | [Embedded manager](manager_embedded.md) |
| `es-fluent-manager-dioxus` | Dioxus client rendering, SSR, or both | [Dioxus manager](manager_dioxus.md) |
| `es-fluent-manager-bevy` | Bevy ECS, assets, and reactive UI text | [Bevy manager](manager_bevy.md) |

All concrete managers follow the same application model:

1. Put `define_i18n_module!()` in a library-reachable module.
2. Keep derived message types reachable from a library target.
3. Initialize or provide a manager context with a selected language.
4. Call `i18n.localize_message(&message)` or
   `MyType::localize_label(&i18n)` for a type label.
5. Use fallible lookup only where the caller intentionally handles a missing
   translation.

Manager macros scan configured locale assets at compile time. Add
`es-fluent-build` to track those assets and produce the fallback-message catalog
used by strict derive validation; see
[Incremental builds](incremental_builds.md).

All managers route typed output through the owning package's missing-message
policy. The default `strict` policy rejects missing fallback message values. Set
`missing_message_policy = "fallback-str"` in `i18n.toml` so normal message and
label lookup returns a snake_case Rust source name after locale fallback is
exhausted; fallible lookup still returns `None`.

Import `es_fluent::FluentLocalizerExt as _` when the context is a generic
`FluentLocalizer` or a trait object, or when calling
`i18n.try_localize_message(&message)`. The method syntax stays the same.
Fallible rendering returns `None` if any lookup in the message tree is missing.

Custom message and backend implementations use `to_fluent_string_with(...)`,
`localize(key, args)`, and `with_lookup(...)` to define rendering and lookup
behavior. Keep these integration contracts inside the adapter; call the typed
methods from application code.

Use `es-fluent-manager-core` directly only when building a custom
runtime integration. Concrete managers provide the intended application-facing
APIs.
