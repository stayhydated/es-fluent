#![doc = include_str!("../README.md")]

pub use unic_langid::{LanguageIdentifier, langid};

#[cfg(feature = "macros")]
pub use es_fluent_lang_macro::es_fluent_language;

use es_fluent_manager_core::{I18nModule, LocalizationError, Localizer};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

const ES_FLUENT_LANG_FTL: &str = include_str!("../es-fluent-lang.ftl");

fn embedded_resource() -> Arc<FluentResource> {
    static RESOURCE: OnceLock<Arc<FluentResource>> = OnceLock::new();
    RESOURCE
        .get_or_init(|| {
            Arc::new(
                FluentResource::try_new(ES_FLUENT_LANG_FTL.to_owned()).expect(
                    "Invalid Fluent resource embedded in es-fluent-lang/es-fluent-lang.ftl",
                ),
            )
        })
        .clone()
}

struct EsFluentLanguageModule;

impl I18nModule for EsFluentLanguageModule {
    fn name(&self) -> &'static str {
        "es-fluent-lang"
    }

    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(EsFluentLanguageLocalizer::new(
            embedded_resource(),
            langid!("en-US"),
        ))
    }
}

struct EsFluentLanguageLocalizer {
    resource: Arc<FluentResource>,
    current_lang: RwLock<LanguageIdentifier>,
}

impl EsFluentLanguageLocalizer {
    fn new(resource: Arc<FluentResource>, default_lang: LanguageIdentifier) -> Self {
        Self {
            resource,
            current_lang: RwLock::new(default_lang),
        }
    }
}

impl Localizer for EsFluentLanguageLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        *self.current_lang.write().expect("lock poisoned") = lang.clone();
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let lang = self.current_lang.read().expect("lock poisoned").clone();
        let mut bundle = FluentBundle::new(vec![lang]);
        if let Err(err) = bundle.add_resource(self.resource.clone()) {
            log::error!("Failed to add es-fluent-lang resource: {:?}", err);
            return None;
        }

        let message = bundle.get_message(id)?;
        let pattern = message.value()?;
        let mut errors = Vec::new();

        let fluent_args = args.map(|args| {
            let mut fluent_args = FluentArgs::new();
            for (key, value) in args {
                fluent_args.set(*key, value.clone());
            }
            fluent_args
        });

        let formatted = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);

        if errors.is_empty() {
            Some(formatted.into_owned())
        } else {
            log::error!(
                "Formatting errors while localizing '{}' from es-fluent-lang: {:?}",
                id,
                errors
            );
            None
        }
    }
}

inventory::submit! {
    &EsFluentLanguageModule as &dyn I18nModule
}

#[cfg(feature = "bevy")]
mod bevy_support {
    use super::*;
    use es_fluent_manager_core::StaticI18nResource;
    use std::sync::Arc;

    struct EsFluentLangStaticResource;

    static STATIC_RESOURCE: EsFluentLangStaticResource = EsFluentLangStaticResource;

    impl StaticI18nResource for EsFluentLangStaticResource {
        fn domain(&self) -> &'static str {
            "es-fluent-lang"
        }

        fn resource(&self) -> Arc<FluentResource> {
            embedded_resource()
        }
    }

    inventory::submit! {
        &STATIC_RESOURCE as &dyn StaticI18nResource
    }
}
