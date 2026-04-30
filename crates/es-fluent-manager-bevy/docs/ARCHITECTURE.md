# es-fluent-manager-bevy Architecture

`es-fluent-manager-bevy` adapts `es-fluent` to Bevy ECS and asset loading without
installing a process-wide localization hook.

## Runtime state

Runtime state lives in Bevy resources:

- `I18nAssets`: asset handles, loaded Fluent resources, and load errors.
- `I18nBundle`: best-effort unscoped per-locale bundles, fallback resources,
  and ready-cache tokens.
- `I18nDomainBundles`: accepted per-domain bundles and fallback resources.
- `I18nResource`: active/request-resolved language state plus a fallback
  `FluentManager` for runtime modules such as `es-fluent-lang`.
- `RequestedLanguageId` and `ActiveLanguageId`: requested user intent versus
  published active locale.
- `BevyI18n`: a `SystemParam` facade over `I18nResource`, `I18nBundle`, and
  `I18nDomainBundles` for system-local explicit localization.

## Localization flow

`FluentText<T>` requires `T: FluentMessage`. Update systems render text through
`BevyI18n`, which implements `FluentLocalizer`:

```rust
let text = i18n.localize_message(&message);
```

Direct user systems use the same context:

```rust
fn update_title(i18n: BevyI18n) {
    let title = i18n.localize_message(&UiMessage::Settings);
}
```

No `es-fluent` global state or custom localizer is installed.

Generated message lookup is domain-scoped. Bundle rebuilds commit
`I18nDomainBundles` independently from the merged all-domain `I18nBundle`, so
duplicate message IDs in separate domains do not block typed generated lookup.
When the merged all-domain bundle is ambiguous, Bevy marks the locale ready for
domain-scoped lookup and leaves raw unscoped lookup without resources for that
locale.

## Asset readiness and runtime fallback managers

The Bevy plugin uses strict module discovery and exposes both
`RequestedLanguageId` and `ActiveLanguageId` so systems can distinguish user
intent from the currently published locale. Failed locale switches keep the last
ready locale active.

When a requested locale falls back to a resolved locale, Bevy publishes the
requested locale for change events and ECS resources while using the resolved
locale for ready bundle lookup. Runtime fallback managers are asked to select
the requested locale first, then the resolved locale. Rejection by the runtime
fallback manager does not block Bevy asset-backed locale publication.
Fallback selection tells `FluentManager` that Bevy assets have already proved
application locale support, so follower-only utility modules can be committed
without making runtime-only locales selectable.

Generated embedded localizers are fallback-aware. Custom runtime localizers that
need parent-locale fallback should implement that behavior in
`select_language(...)`.

Only metadata-only Bevy registrations create Bevy asset availability. Runtime
localizer registrations are reserved for the fallback manager and do not make a
locale wait on Bevy asset bundles. Runtime fallback managers are attached when
runtime modules are discovered, even when startup locale selection is rejected.
In that case they remain attached with no selected runtime localizers until a
later accepted locale switch. Runtime fallback managers are used only after Bevy
resolves a locale through asset or ready-bundle availability during startup or a
later `LocaleChangeEvent`. Runtime-only locales do not by themselves make a
Bevy locale switch selectable.

## Startup

`I18nPlugin` performs strict module discovery, initializes resources, attaches a
runtime fallback manager to `I18nResource`, registers discovered `BevyFluentText`
types, and configures asset/bundle/locale systems.

## Locale switching

Locale change events resolve the requested locale against ready, available, and
blocked assets. Accepted switches update `I18nResource`, `ActiveLanguageId`, and
locale events. Failed switches keep the last ready locale active.

When the requested locale has an available fallback that is not ready yet, the
plugin keeps a pending language change and applies it after the bundle becomes
ready. Current-locale hot reloads re-emit `LocaleChangedEvent` only after the
replacement bundle is accepted.

`RefreshForLocale` receives the requested locale stored in
`ActiveLanguageId`, even when asset lookup resolves through a parent fallback
bundle. This keeps locale-aware fields such as generated language enums aligned
with user intent.
