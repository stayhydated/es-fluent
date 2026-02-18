#![doc = include_str!("../README.md")]

pub use unic_langid::{LanguageIdentifier, langid};

#[cfg(feature = "macros")]
pub use es_fluent_lang_macro::es_fluent_language;

#[doc(hidden)]
use es_fluent_manager_core::{I18nModule, LocalizationError, Localizer};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
use rust_embed::RustEmbed;
#[cfg(feature = "localized-langs")]
use std::collections::HashSet;

#[cfg(not(feature = "localized-langs"))]
const ES_FLUENT_LANG_FTL: &str = include_str!("../es-fluent-lang.ftl");

#[cfg(not(feature = "localized-langs"))]
#[doc(hidden)]
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

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
const I18N_RESOURCE_NAME: &str = "es-fluent-lang.ftl";

#[cfg(feature = "localized-langs")]
#[derive(RustEmbed)]
#[folder = "i18n"]
#[doc(hidden)]
struct EsFluentLangAssets;

#[cfg(all(feature = "bevy", not(feature = "localized-langs")))]
#[allow(dead_code)]
#[doc(hidden)]
const AUTONYM_FTL: &str = include_str!("../es-fluent-lang.ftl");

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
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

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
fn candidate_languages(lang: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    use es_fluent_manager_core::locale_candidates;

    locale_candidates(lang)
}

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
fn resolve_language(lang: &LanguageIdentifier) -> Option<LanguageIdentifier> {
    let available = available_languages();
    candidate_languages(lang)
        .into_iter()
        .find(|candidate| available.contains(candidate))
}

fn build_fluent_args<'a>(args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<FluentArgs<'a>> {
    args.map(|args| {
        let mut fluent_args = FluentArgs::new();
        for (key, value) in args {
            fluent_args.set((*key).to_string(), value.clone());
        }
        fluent_args
    })
}

fn localize_from_resource<'a>(
    lang: LanguageIdentifier,
    resource: Arc<FluentResource>,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<String> {
    let mut bundle = FluentBundle::new(vec![lang]);
    if let Err(err) = bundle.add_resource(resource) {
        tracing::error!("Failed to add es-fluent-lang resource: {:?}", err);
        return None;
    }

    let message = bundle.get_message(id)?;
    let pattern = message.value()?;
    let mut errors = Vec::new();
    let fluent_args = build_fluent_args(args);
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

#[doc(hidden)]
struct EsFluentLanguageModule;

impl I18nModule for EsFluentLanguageModule {
    fn name(&self) -> &'static str {
        "es-fluent-lang"
    }

    fn create_localizer(&self) -> Box<dyn Localizer> {
        #[cfg(not(feature = "localized-langs"))]
        {
            Box::new(EsFluentLanguageLocalizer::new(
                embedded_resource(),
                langid!("en-US"),
            ))
        }

        #[cfg(feature = "localized-langs")]
        {
            Box::new(EsFluentLanguageLocalizer::new(langid!("en-US")))
        }
    }
}

#[cfg(not(feature = "localized-langs"))]
#[doc(hidden)]
struct EsFluentLanguageLocalizer {
    resource: Arc<FluentResource>,
    current_lang: RwLock<LanguageIdentifier>,
}

#[cfg(not(feature = "localized-langs"))]
#[doc(hidden)]
impl EsFluentLanguageLocalizer {
    fn new(resource: Arc<FluentResource>, default_lang: LanguageIdentifier) -> Self {
        Self {
            resource,
            current_lang: RwLock::new(default_lang),
        }
    }
}

#[cfg(not(feature = "localized-langs"))]
#[doc(hidden)]
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
        localize_from_resource(lang, self.resource.clone(), id, args)
    }
}

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
struct EsFluentLanguageLocalizer {
    resources: RwLock<HashMap<LanguageIdentifier, Arc<FluentResource>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
}

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
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

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
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

        localize_from_resource(lang, resource, id, args)
    }
}

inventory::submit! {
    &EsFluentLanguageModule as &dyn I18nModule
}

#[cfg(all(feature = "bevy", not(feature = "localized-langs")))]
#[doc(hidden)]
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

#[cfg(all(feature = "bevy", feature = "localized-langs"))]
#[doc(hidden)]
mod bevy_support {
    use super::*;
    use es_fluent_manager_core::StaticI18nResource;
    use std::sync::Arc;

    pub(super) struct EsFluentLangStaticResource {
        locale: &'static str,
        language: OnceLock<Option<LanguageIdentifier>>,
        resource: OnceLock<Option<Arc<FluentResource>>>,
    }

    impl EsFluentLangStaticResource {
        pub const fn new(locale: &'static str) -> Self {
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
            self.resource
                .get_or_init(|| {
                    let locale = self.language()?;
                    let path = format!("{}/{}", locale, I18N_RESOURCE_NAME);
                    let file = EsFluentLangAssets::get(&path)?;
                    let content = String::from_utf8(file.data.to_vec()).ok()?;
                    let resource = FluentResource::try_new(content).ok()?;
                    Some(Arc::new(resource))
                })
                .clone()
        }
    }

    impl StaticI18nResource for EsFluentLangStaticResource {
        fn domain(&self) -> &'static str {
            "es-fluent-lang"
        }

        fn matches_language(&self, lang: &LanguageIdentifier) -> bool {
            self.language().is_some_and(|l| l == lang)
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

#[cfg(all(test, feature = "bevy", not(feature = "localized-langs")))]
mod bevy_static_resource_tests {
    use es_fluent_manager_core::StaticI18nResource;
    use inventory::iter;

    #[test]
    fn with_bevy_feature_static_resource_is_registered() {
        let resources: Vec<_> = iter::<&dyn StaticI18nResource>()
            .filter(|r| r.domain() == "es-fluent-lang")
            .collect();
        assert!(
            !resources.is_empty(),
            "Expected es-fluent-lang static resource with bevy feature"
        );
    }
}

#[cfg(all(test, not(feature = "bevy")))]
mod static_resource_tests {
    use es_fluent_manager_core::StaticI18nResource;
    use inventory::iter;

    #[test]
    fn without_localized_langs_feature_no_static_resources_registered() {
        let resources: Vec<_> = iter::<&dyn StaticI18nResource>()
            .filter(|r| r.domain() == "es-fluent-lang")
            .collect();
        assert!(
            resources.is_empty(),
            "Expected no es-fluent-lang static resources without localized-langs feature, but found {}",
            resources.len()
        );
    }
}

#[cfg(all(test, not(feature = "localized-langs")))]
mod tests {
    use super::*;
    use unic_langid::langid;

    #[test]
    fn embedded_resource_is_cached_and_localizes_known_keys() {
        let first = embedded_resource();
        let second = embedded_resource();
        assert!(Arc::ptr_eq(&first, &second));

        assert_eq!(
            localize_from_resource(langid!("en-US"), first, "es-fluent-lang-en", None),
            Some("English".to_string())
        );
        assert_eq!(
            localize_from_resource(langid!("en-US"), second, "missing-key", None),
            None
        );
    }

    #[test]
    fn localize_from_resource_formats_args_and_reports_missing_args() {
        let resource = Arc::new(
            FluentResource::try_new("welcome = Welcome, { $name }!".to_string())
                .expect("valid ftl"),
        );

        assert_eq!(
            localize_from_resource(langid!("en-US"), resource.clone(), "welcome", None),
            None
        );

        let mut args = HashMap::new();
        args.insert("name", FluentValue::from("Mark"));
        let localized = localize_from_resource(langid!("en-US"), resource, "welcome", Some(&args));
        assert!(
            localized
                .as_deref()
                .is_some_and(|value| value.contains("Welcome"))
        );
        assert!(
            localized
                .as_deref()
                .is_some_and(|value| value.contains("Mark"))
        );
    }

    #[test]
    fn language_module_creates_localizer_and_selects_language() {
        let module = EsFluentLanguageModule;
        assert_eq!(module.name(), "es-fluent-lang");

        let localizer = module.create_localizer();
        localizer
            .select_language(&langid!("en-US"))
            .expect("language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("français".to_string())
        );
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn bevy_static_resource_is_registered_for_es_fluent_lang() {
        use es_fluent_manager_core::StaticI18nResource;

        let resource = inventory::iter::<&'static dyn StaticI18nResource>()
            .find(|resource| resource.domain() == "es-fluent-lang")
            .expect("es-fluent-lang static resource should be registered");

        assert!(resource.matches_language(&langid!("en-US")));
        assert!(resource.resource().get_entry(0).is_some());
    }
}

#[cfg(all(test, feature = "localized-langs"))]
mod tests_localized {
    use super::*;
    use unic_langid::langid;

    #[test]
    fn localize_from_resource_formats_args_and_reports_missing_args() {
        let resource = Arc::new(
            FluentResource::try_new("welcome = Welcome, { $name }!".to_string())
                .expect("valid ftl"),
        );

        assert_eq!(
            localize_from_resource(langid!("en-US"), resource.clone(), "welcome", None),
            None
        );

        let mut args = HashMap::new();
        args.insert("name", FluentValue::from("Mark"));
        let localized = localize_from_resource(langid!("en-US"), resource, "welcome", Some(&args));
        assert!(
            localized
                .as_deref()
                .is_some_and(|value| value.contains("Welcome"))
        );
        assert!(
            localized
                .as_deref()
                .is_some_and(|value| value.contains("Mark"))
        );
    }

    #[test]
    fn language_module_creates_localizer_and_selects_language() {
        let module = EsFluentLanguageModule;
        assert_eq!(module.name(), "es-fluent-lang");

        let localizer = module.create_localizer();
        localizer
            .select_language(&langid!("fr"))
            .expect("language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("français".to_string())
        );
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn bevy_static_resource_is_registered_for_es_fluent_lang() {
        use es_fluent_manager_core::StaticI18nResource;

        let resources: Vec<_> = inventory::iter::<&'static dyn StaticI18nResource>()
            .filter(|r| r.domain() == "es-fluent-lang")
            .collect();

        assert!(
            !resources.is_empty(),
            "Expected es-fluent-lang static resources"
        );

        let en_resource = inventory::iter::<&'static dyn StaticI18nResource>()
            .find(|r| r.domain() == "es-fluent-lang" && r.matches_language(&langid!("en")));

        assert!(
            en_resource.is_some(),
            "Expected es-fluent-lang static resource for 'en' locale"
        );
        assert!(en_resource.unwrap().resource().get_entry(0).is_some());
    }
}
