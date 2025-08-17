// In: crates/es-fluent-macros/src/lib.rs

use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn define_i18n_module(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    let crate_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set");

    let localizer_struct_name = syn::Ident::new(&format!("{}Localizer", &crate_name.to_upper_camel_case()), proc_macro2::Span::call_site());
    let module_struct_name = syn::Ident::new(&format!("{}I18nModule", &crate_name.to_upper_camel_case()), proc_macro2::Span::call_site());

    let expanded = quote! {
        // Encapsulate implementation details in a private module.
        mod __es_fluent_generated {
            use es_fluent::{Localizer, LocalizationError};
            use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
            use fluent_bundle::concurrent::FluentBundle as ConcurrentFluentBundle;
            use std::collections::HashMap;
            use std::sync::{Arc, Mutex};
            use unic_langid::LanguageIdentifier;

            #[derive(rust_embed::RustEmbed)]
            #[folder = #path]
            struct Localizations;

            pub struct #localizer_struct_name {
                bundle: Arc<Mutex<Option<ConcurrentFluentBundle<FluentResource>>>>,
            }

            impl #localizer_struct_name {
                pub fn new(fallback_language: LanguageIdentifier) -> Self {
                    let bundle = Self::create_bundle(fallback_language.clone())
                        .expect("Failed to load this module's fallback language from embedded assets.");
                    Self { bundle: Arc::new(Mutex::new(Some(bundle))) }
                }

                fn create_bundle(lang: LanguageIdentifier) -> Option<ConcurrentFluentBundle<FluentResource>> {
                  // Explicitly create a bundle with a concurrent memoizer to ensure Send + Sync
                  let mut bundle = ConcurrentFluentBundle::<FluentResource>::new_concurrent(vec![lang.clone()]);
                  let ftl_path = format!("{}/{}.ftl", lang, #crate_name);

                  let file = Localizations::get(&ftl_path)
                    .unwrap_or_else(|| panic!("FTL file '{}' not found in embedded assets", ftl_path));
                  let content = std::str::from_utf8(file.data.as_ref()).ok()?;
                  let res = FluentResource::try_new(content.to_string()).expect("Failed to parse FTL file.");
                  bundle.add_resource(res).ok()?;
                  Some(bundle)
                }
            }

            impl Localizer for #localizer_struct_name {
                fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
                    if let Some(bundle) = Self::create_bundle(lang.clone()) {
                        let mut guard = self.bundle.lock().unwrap();
                        *guard = Some(bundle);
                        Ok(())
                    } else {
                        Err(LocalizationError::LanguageNotSupported(lang.clone()))
                    }
                }

                fn localize<'a>(&self, id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String> {
                    let guard = self.bundle.lock().unwrap();
                    let bundle = guard.as_ref()?;
                    let msg = bundle.get_message(id)?;
                    let pattern = msg.value()?;
                    let mut errors = Vec::new();

                    let fluent_args = args.map(|args| {
                        let mut fa = FluentArgs::new();
                        for (key, value) in args {
                            fa.set(*key, value.clone());
                        }
                        fa
                    });

                    let value = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);
                    if !errors.is_empty() {
                       log::error!("Fluent formatting errors for message '{}': {:?}", id, errors);
                    }
                    Some(value.to_string())
                }
            }
        }

        // The public descriptor that gets registered.
        struct #module_struct_name;

        impl es_fluent::I18nModule for #module_struct_name {
            fn name(&self) -> &'static str { #crate_name }

            fn create_localizer(&self) -> Box<dyn es_fluent::Localizer> {
                // TODO: This should ideally read the fallback language from i18n.toml.
                // For now, we hardcode a common default.
                let fallback_lang = unic_langid::langid!("en");
                Box::new(self::__es_fluent_generated::#localizer_struct_name::new(fallback_lang))
            }
        }

        inventory::submit!(&#module_struct_name as &dyn es_fluent::I18nModule);
    };
    TokenStream::from(expanded)
}
