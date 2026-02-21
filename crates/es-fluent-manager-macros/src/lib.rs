#![doc = include_str!("../README.md")]

mod assets;
mod bevy_fluent_text;
mod module_macros;

use proc_macro::TokenStream;

/// Defines an embedded i18n module.
///
/// This macro will:
///
/// 1.  Read the `i18n.toml` configuration file.
/// 2.  Discover the available languages in the `i18n` directory.
/// 3.  Generate a `RustEmbed` struct for the i18n assets.
/// 4.  Generate an `EmbeddedI18nModule` for the crate.
#[proc_macro]
pub fn define_embedded_i18n_module(input: TokenStream) -> TokenStream {
    module_macros::define_embedded_i18n_module(input)
}

/// Defines a Bevy i18n module.
///
/// This macro will:
///
/// 1.  Read the `i18n.toml` configuration file.
/// 2.  Discover the available languages in the `i18n` directory.
/// 3.  Generate a metadata descriptor and language resource manifest for the crate.
#[proc_macro]
pub fn define_bevy_i18n_module(input: TokenStream) -> TokenStream {
    module_macros::define_bevy_i18n_module(input)
}

/// Registers a type for use with `FluentText<T>` in Bevy.
///
/// This derive macro auto-registers the type with `I18nPlugin` so you don't need
/// to manually call `app.register_fluent_text::<T>()`.
///
/// If any fields are marked with `#[locale]`, the macro will:
/// - Auto-generate a `RefreshForLocale` implementation
/// - Use `register_fluent_text_from_locale` instead of `register_fluent_text`
///
/// The `#[locale]` attribute marks fields that should be updated when the locale changes.
/// The field type must implement `TryFrom<&LanguageIdentifier>`.
///
/// # Example (simple)
///
/// ```ignore
/// use es_fluent::EsFluent;
/// use es_fluent_manager_bevy::BevyFluentText;
/// use bevy::prelude::Component;
///
/// #[derive(BevyFluentText, Clone, Component, EsFluent)]
/// pub enum ButtonState {
///     Normal,
///     Hovered,
///     Pressed,
/// }
/// ```
///
/// # Example (with locale refresh)
///
/// ```ignore
/// use es_fluent::EsFluent;
/// use es_fluent_manager_bevy::BevyFluentText;
/// use bevy::prelude::Component;
///
/// #[derive(BevyFluentText, Clone, Component, EsFluent)]
/// pub enum ScreenMessages {
///     ToggleLanguageHint {
///         key: KbKeys,
///         #[locale]
///         current_language: Languages,
///     },
/// }
/// ```
#[proc_macro_derive(BevyFluentText, attributes(locale))]
pub fn derive_bevy_fluent_text(input: TokenStream) -> TokenStream {
    bevy_fluent_text::derive_bevy_fluent_text(input)
}
