# es-fluent-manager-dioxus architecture

`es-fluent-manager-dioxus` adapts `es-fluent` localization to Dioxus through
Dioxus asset loading and explicit component/request context.

## Runtime surfaces

The crate has no default runtime feature.

- `client` enables provider and hook APIs for interactive Dioxus rendering.
- `ssr` enables request-scoped server-side rendering helpers.
- `define_i18n_module!` is always available and emits Dioxus asset module
  metadata for the current crate.

## Asset module generation

`define_i18n_module!` scans the same `i18n.toml` layout used by the other
managers and emits Dioxus `asset!` handles for each discovered FTL file. The
generated code includes:

- static `ModuleData`,
- one `DioxusI18nAssetResource` per discovered FTL file,
- one static `DioxusI18nAssetModule`,
- `dioxus_i18n_asset_modules()` for provider/runtime initialization,
- `load_dioxus_i18n_assets(...)` helpers for explicit async loading.

Dioxus `asset!` resolves paths relative to the package root and rejects files
outside that root, so Dioxus manager assets must be package-local.

## Loaded localizer

`DioxusAssetI18n::load_modules(...)` reads generated asset handles through the
Dioxus asset resolver on WASM targets. On non-WASM targets it reads the
`asset!` source path directly, which avoids requiring a Tokio runtime during
static generation and server-side tests. Loaded bytes are parsed into
`FluentResource`s and stored in a cloneable explicit localizer.

Language selection mirrors the embedded manager behavior after loading:
best-effort selection keeps modules that support the requested locale, strict
selection requires every generated module to support it, and localization can
fall back through available locale resources.

After asset-backed locale support is established, the manager asks discovered
runtime follower modules to select the same locale. Follower modules are runtime
localizers that return `false` from `contributes_to_language_selection()`, such
as `es-fluent-lang`; they provide utility lookups without making locales
selectable by themselves. Application translations remain Dioxus asset-backed.

## Client context

`DioxusAssetI18nProvider` wraps the asset load in a Dioxus resource. It renders
`loading` while pending, renders `fallback` on load failure, and installs a
ready `DioxusAssetI18nHandle` context once loading succeeds.

`DioxusAssetI18nHandle::localize_message(...)` reads the tracked language
signal before delegating to the loaded localizer. Language changes route through
`select_language(...)` or `select_language_strict(...)`; both update the signal
only after the localizer accepts the switch.

## SSR runtime

`SsrI18nRuntime` is constructed from the generated `DioxusI18nAssetModules`
handle. Each request loads a fresh `DioxusAssetI18n`, so request language state
is isolated. `request(...)` is async because Dioxus asset reads are async;
blocking wrappers exist for static generation and sync SSR entry points.

Components receive `SsrI18n` explicitly, usually as a prop or through
app-owned context, and call `localize_message(...)` or typed label helpers.
