use heck::ToUpperCamelCase as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};

#[proc_macro]
pub fn register_i18n_module(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);

    let crate_name = std::env::var("CARGO_PKG_NAME")
        .expect("the CARGO_PKG_NAME environment variable must be set");
    let struct_name = syn::Ident::new(
        &format!("{}i18nModule", &crate_name.to_upper_camel_case()),
        proc_macro2::Span::call_site(),
    );

    let expanded = quote! {
        #[derive(rust_embed::RustEmbed)]
        #[folder = #path]
        struct Localizations;

        pub static LANGUAGE_LOADER: std::sync::LazyLock<i18n_embed::fluent::FluentLanguageLoader> = std::sync::LazyLock::new(|| {
            use i18n_embed::LanguageLoader as _;
            let loader = i18n_embed::fluent::fluent_language_loader!();

            loader
                .load_fallback_language(&Localizations)
                .expect("Error while loading fallback language");

            #[cfg(test)]
            loader.set_use_isolating(false);

            loader
        });

        #[macro_export]
        macro_rules! fl {
            ($message_id:literal) => {{
                i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id)
            }};

            ($message_id:literal, $($args:expr),*) => {{
                i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id, $($args), *)
            }};
        }

        #[must_use]
        pub fn localizer() -> Box<dyn i18n_embed::Localizer> {
            Box::from(i18n_embed::DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations))
        }

        struct #struct_name;

        impl es_fluent_manager_bevy::I18nModule for #struct_name {
            fn name(&self) -> &'static str {
                env!("CARGO_PKG_NAME")
            }

            fn init(&self, requested_languages: &[i18n_embed::unic_langid::LanguageIdentifier]) -> Result<(), es_fluent_manager_bevy::I18nManagerError> {
            let _ = localizer().select(requested_languages)?;
            Ok(())
            }

            fn change_locale(&self, language: &str) -> Result<(), es_fluent_manager_bevy::I18nManagerError> {
            let lang_id: i18n_embed::unic_langid::LanguageIdentifier = language.parse()
                .map_err(|e| es_fluent_manager_bevy::I18nManagerError::ModuleError {
                    module_name: self.name().to_string(),
                    source: anyhow::anyhow!("Failed to parse language identifier: {}", e),
                })?;

            let requested_languages = vec![lang_id];

            localizer().select(&requested_languages)?;

            Ok(())
            }
        }

        inventory::submit!(&#struct_name as &dyn es_fluent_manager_bevy::I18nModule);

    };

    TokenStream::from(expanded)
}
