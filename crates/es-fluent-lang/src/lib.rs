#![doc = include_str!("../README.md")]

pub use unic_langid::{LanguageIdentifier, langid};

#[cfg(feature = "macros")]
pub use es_fluent_lang_macro::es_fluent_language;

use es_fluent_manager_core::{I18nModule, LocalizationError, Localizer};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(not(feature = "minimal"))]
use rust_embed::RustEmbed;
#[cfg(not(feature = "minimal"))]
use std::collections::HashSet;

#[cfg(feature = "minimal")]
const ES_FLUENT_LANG_FTL: &str = include_str!("../es-fluent-lang.ftl");

#[cfg(feature = "minimal")]
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

#[cfg(not(feature = "minimal"))]
const I18N_RESOURCE_NAME: &str = "es-fluent-lang.ftl";

#[cfg(not(feature = "minimal"))]
#[derive(RustEmbed)]
#[folder = "i18n"]
struct EsFluentLangAssets;

#[cfg(not(feature = "minimal"))]
fn available_languages() -> &'static HashSet<LanguageIdentifier> {
    static AVAILABLE: OnceLock<HashSet<LanguageIdentifier>> = OnceLock::new();
    AVAILABLE.get_or_init(|| {
        let mut set = HashSet::new();
        for file in EsFluentLangAssets::iter() {
            let path = file.as_ref();
            if let Some((lang, file_name)) = path.rsplit_once('/')
                && file_name == I18N_RESOURCE_NAME
                && let Ok(lang_id) = lang.parse::<LanguageIdentifier>()
            {
                set.insert(lang_id);
            }
        }
        set
    })
}

#[cfg(not(feature = "minimal"))]
fn candidate_languages(lang: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    let mut candidates = Vec::new();
    let mut push = |candidate: LanguageIdentifier| {
        if !candidates.iter().any(|existing| existing == &candidate) {
            candidates.push(candidate);
        }
    };

    push(lang.clone());

    let mut without_variants = lang.clone();
    without_variants.clear_variants();
    push(without_variants.clone());

    if without_variants.region.is_some() {
        let mut no_region = without_variants.clone();
        no_region.region = None;
        push(no_region);
    }

    if without_variants.script.is_some() {
        let mut no_script = without_variants.clone();
        no_script.script = None;
        push(no_script);
    }

    if let Ok(primary) = without_variants
        .language
        .as_str()
        .parse::<LanguageIdentifier>()
    {
        push(primary);
    }

    candidates
}

#[cfg(not(feature = "minimal"))]
fn resolve_language(lang: &LanguageIdentifier) -> Option<LanguageIdentifier> {
    let available = available_languages();
    candidate_languages(lang)
        .into_iter()
        .find(|candidate| available.contains(candidate))
}

struct EsFluentLanguageModule;

impl I18nModule for EsFluentLanguageModule {
    fn name(&self) -> &'static str {
        "es-fluent-lang"
    }

    fn create_localizer(&self) -> Box<dyn Localizer> {
        #[cfg(feature = "minimal")]
        {
            Box::new(EsFluentLanguageLocalizer::new(
                embedded_resource(),
                langid!("en-US"),
            ))
        }

        #[cfg(not(feature = "minimal"))]
        {
            Box::new(EsFluentLanguageLocalizer::new(langid!("en-US")))
        }
    }
}

#[cfg(feature = "minimal")]
struct EsFluentLanguageLocalizer {
    resource: Arc<FluentResource>,
    current_lang: RwLock<LanguageIdentifier>,
}

#[cfg(feature = "minimal")]
impl EsFluentLanguageLocalizer {
    fn new(resource: Arc<FluentResource>, default_lang: LanguageIdentifier) -> Self {
        Self {
            resource,
            current_lang: RwLock::new(default_lang),
        }
    }
}

#[cfg(feature = "minimal")]
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
            tracing::error!("Failed to add es-fluent-lang resource: {:?}", err);
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
            tracing::error!(
                "Formatting errors while localizing '{}' from es-fluent-lang: {:?}",
                id,
                errors
            );
            None
        }
    }
}

#[cfg(not(feature = "minimal"))]
struct EsFluentLanguageLocalizer {
    resources: RwLock<HashMap<LanguageIdentifier, Arc<FluentResource>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
}

#[cfg(not(feature = "minimal"))]
impl EsFluentLanguageLocalizer {
    fn new(default_lang: LanguageIdentifier) -> Self {
        let localizer = Self {
            resources: RwLock::new(HashMap::new()),
            current_lang: RwLock::new(None),
        };
        let _ = localizer.set_language(&default_lang);
        localizer
    }

    fn set_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let resolved = resolve_language(lang)
            .ok_or_else(|| LocalizationError::LanguageNotSupported(lang.clone()))?;
        let _ = self.load_resource(&resolved)?;
        *self.current_lang.write().expect("lock poisoned") = Some(resolved);
        Ok(())
    }

    fn load_resource(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<Arc<FluentResource>, LocalizationError> {
        if let Some(resource) = self
            .resources
            .read()
            .expect("lock poisoned")
            .get(lang)
            .cloned()
        {
            return Ok(resource);
        }

        let path = format!("{}/{}", lang, I18N_RESOURCE_NAME);
        let file = EsFluentLangAssets::get(&path)
            .ok_or_else(|| LocalizationError::LanguageNotSupported(lang.clone()))?;
        let content = match String::from_utf8(file.data.to_vec()) {
            Ok(content) => content,
            Err(err) => {
                tracing::error!("Invalid UTF-8 in embedded file '{}': {}", path, err);
                String::from_utf8_lossy(err.as_bytes()).into_owned()
            },
        };
        let resource = FluentResource::try_new(content)
            .map_err(|(_, errs)| LocalizationError::FluentParseError(errs))?;
        let resource = Arc::new(resource);
        self.resources
            .write()
            .expect("lock poisoned")
            .insert(lang.clone(), resource.clone());
        Ok(resource)
    }
}

#[cfg(not(feature = "minimal"))]
impl Localizer for EsFluentLanguageLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        self.set_language(lang)
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let lang = self.current_lang.read().expect("lock poisoned").clone()?;
        let resource = match self.load_resource(&lang) {
            Ok(resource) => resource,
            Err(err) => {
                tracing::error!("Failed to load es-fluent-lang resource: {}", err);
                return None;
            },
        };

        let mut bundle = FluentBundle::new(vec![lang]);
        if let Err(err) = bundle.add_resource(resource) {
            tracing::error!("Failed to add es-fluent-lang resource: {:?}", err);
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
            tracing::error!(
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

#[cfg(all(feature = "bevy", feature = "minimal"))]
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

#[cfg(all(feature = "bevy", not(feature = "minimal")))]
mod bevy_support {
    use super::*;
    use es_fluent_manager_core::StaticI18nResource;
    use std::sync::Arc;

    struct EsFluentLangStaticResource {
        locale: &'static str,
        language: OnceLock<Option<LanguageIdentifier>>,
        resource: OnceLock<Option<Arc<FluentResource>>>,
    }

    impl EsFluentLangStaticResource {
        const fn new(locale: &'static str) -> Self {
            Self {
                locale,
                language: OnceLock::new(),
                resource: OnceLock::new(),
            }
        }

        fn language(&self) -> Option<&LanguageIdentifier> {
            self.language
                .get_or_init(|| self.locale.parse().ok())
                .as_ref()
        }

        fn load_resource(&self) -> Option<Arc<FluentResource>> {
            let path = format!("{}/{}", self.locale, I18N_RESOURCE_NAME);
            let resource = self.resource.get_or_init(|| {
                let file = EsFluentLangAssets::get(&path)?;
                let content = match String::from_utf8(file.data.to_vec()) {
                    Ok(content) => content,
                    Err(err) => {
                        tracing::error!("Invalid UTF-8 in embedded file '{}': {}", path, err);
                        String::from_utf8_lossy(err.as_bytes()).into_owned()
                    },
                };
                let resource = match FluentResource::try_new(content) {
                    Ok(resource) => resource,
                    Err((_, errs)) => {
                        tracing::error!(
                            "Failed to parse fluent resource from '{}': {:?}",
                            path,
                            errs
                        );
                        return None;
                    },
                };
                Some(Arc::new(resource))
            });
            resource.clone()
        }
    }

    impl StaticI18nResource for EsFluentLangStaticResource {
        fn domain(&self) -> &'static str {
            "es-fluent-lang"
        }

        fn matches_language(&self, lang: &LanguageIdentifier) -> bool {
            let Some(resolved) = resolve_language(lang) else {
                return false;
            };
            let Some(candidate) = self.language() else {
                return false;
            };
            if candidate != &resolved {
                return false;
            }
            self.load_resource().is_some()
        }

        fn resource(&self) -> Arc<FluentResource> {
            self.load_resource().unwrap_or_else(|| {
                Arc::new(
                    FluentResource::try_new(String::new())
                        .expect("Empty fluent resource should parse"),
                )
            })
        }
    }

    include!(concat!(
        env!("OUT_DIR"),
        "/es_fluent_lang_static_resources.rs"
    ));
}
