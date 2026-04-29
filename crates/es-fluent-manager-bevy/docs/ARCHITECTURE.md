# es-fluent-manager-bevy Architecture

`es-fluent-manager-bevy` adapts `es-fluent` to Bevy ECS and asset loading without
installing a process-wide localization hook.

## Runtime state

Runtime state lives in Bevy resources:

- `I18nAssets`: asset handles, loaded Fluent resources, and load errors.
- `I18nBundle`: accepted per-locale bundles and fallback resources.
- `I18nDomainBundles`: accepted per-domain resources.
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
