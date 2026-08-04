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
4. Call `localize_message(&message)` or pass that context to typed
   label helpers.
5. Use fallible lookup only where the caller intentionally handles a missing
   translation.

Manager macros scan configured locale assets at compile time. Add
`es-fluent-build` tracking when files or locale folders can change;
see [Incremental builds](incremental_builds.md).

Use `es-fluent-manager-core` directly only when building a custom
runtime integration. Concrete managers provide the intended application-facing
APIs.
