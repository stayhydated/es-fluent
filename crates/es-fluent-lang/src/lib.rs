#![doc = include_str!("../README.md")]

pub use unic_langid::{LanguageIdentifier, langid};

/// Force the linker to keep `es-fluent-lang` runtime resources.
///
/// This is used by WASM examples where localization inventory registration can be
/// stripped by aggressive release optimization.
#[doc(hidden)]
#[cfg(feature = "bevy")]
#[used]
static FORCE_LINK_KEEPALIVE: fn() -> usize = bevy_support::force_link;

#[doc(hidden)]
#[inline(never)]
pub fn force_link() -> usize {
    #[cfg(feature = "bevy")]
    {
        bevy_support::force_link()
    }

    #[cfg(not(feature = "bevy"))]
    {
        0
    }
}

#[cfg(feature = "macros")]
pub use es_fluent_lang_macro::es_fluent_language;

#[doc(hidden)]
use es_fluent_manager_core::{
    I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
    ModuleData, localize_with_bundle, resolve_fallback_language,
};
use fluent_bundle::{FluentBundle, FluentResource, FluentValue};
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
                && lang == lang_id.to_string()
            {
                set.insert(lang_id);
            }
        }
        set
    })
}

#[cfg(feature = "localized-langs")]
#[doc(hidden)]
fn resolve_language(lang: &LanguageIdentifier) -> Option<LanguageIdentifier> {
    let available = available_languages().iter().cloned().collect::<Vec<_>>();
    resolve_fallback_language(lang, &available)
}

#[cfg(feature = "localized-langs")]
fn parse_embedded_resource(
    path: &str,
    bytes: &[u8],
) -> Result<Arc<FluentResource>, LocalizationError> {
    let content = String::from_utf8(bytes.to_vec()).map_err(|err| {
        tracing::error!("Invalid UTF-8 in embedded file '{}': {}", path, err);
        LocalizationError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid UTF-8 in embedded file '{}': {}", path, err),
        ))
    })?;
    let resource = FluentResource::try_new(content)
        .map_err(|(_, errs)| LocalizationError::FluentParseError(errs))?;
    Ok(Arc::new(resource))
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

    let (formatted, errors) = localize_with_bundle(&bundle, id, args)?;

    if errors.is_empty() {
        Some(formatted)
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

static ES_FLUENT_LANG_MODULE_DATA: ModuleData = ModuleData {
    name: "es-fluent-lang",
    domain: "es-fluent-lang",
    supported_languages: &[],
    namespaces: &[],
};

impl I18nModuleDescriptor for EsFluentLanguageModule {
    fn data(&self) -> &'static ModuleData {
        &ES_FLUENT_LANG_MODULE_DATA
    }
}

impl I18nModule for EsFluentLanguageModule {
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
        *self.current_lang.write().expect("lock poisoned") = Some(lang.clone());
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
        let resource = parse_embedded_resource(&path, file.data.as_ref())?;
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
        let requested_lang = self.current_lang.read().expect("lock poisoned").clone()?;
        let resolved_lang = match resolve_language(&requested_lang) {
            Some(lang) => lang,
            None => {
                tracing::error!(
                    "Failed to resolve es-fluent-lang resource locale for '{}'",
                    requested_lang
                );
                return None;
            },
        };
        let resource = match self.load_resource(&resolved_lang) {
            Ok(resource) => resource,
            Err(err) => {
                tracing::error!("Failed to load es-fluent-lang resource: {}", err);
                return None;
            },
        };

        localize_from_resource(requested_lang, resource, id, args)
    }
}

inventory::submit! {
    &ES_FLUENT_LANGUAGE_MODULE as &dyn I18nModuleRegistration
}

static ES_FLUENT_LANGUAGE_MODULE: EsFluentLanguageModule = EsFluentLanguageModule;

#[cfg(feature = "bevy")]
#[doc(hidden)]
mod bevy_support {
    use super::*;

    pub(crate) fn force_link() -> usize {
        let module: &'static dyn I18nModuleRegistration = &ES_FLUENT_LANGUAGE_MODULE;
        let _ = module.create_localizer();
        usize::from(!module.data().domain.is_empty())
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
        assert_eq!(module.data().name, "es-fluent-lang");

        let localizer = I18nModule::create_localizer(&module);
        localizer
            .select_language(&langid!("en-US"))
            .expect("language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("français".to_string())
        );
    }

    #[test]
    fn autonym_mode_returns_native_language_names_regardless_of_selected_locale() {
        let module = EsFluentLanguageModule;
        let localizer = I18nModule::create_localizer(&module);

        localizer
            .select_language(&langid!("en-US"))
            .expect("language selection should succeed");

        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("English".to_string()),
            "English autonym should be 'English'"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("français".to_string()),
            "French autonym should be 'français' (native script)"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-ja", None),
            Some("日本語".to_string()),
            "Japanese autonym should be '日本語' (native script)"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-de", None),
            Some("Deutsch".to_string()),
            "German autonym should be 'Deutsch' (native script)"
        );

        localizer
            .select_language(&langid!("ja-JP"))
            .expect("language selection should succeed");

        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("français".to_string()),
            "Autonym mode should still return 'français' even with Japanese locale selected"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-ja", None),
            Some("日本語".to_string()),
            "Autonym mode should still return '日本語' even with Japanese locale selected"
        );
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn bevy_uses_standard_module_registration_for_autonyms() {
        let registration = inventory::iter::<&'static dyn I18nModuleRegistration>()
            .find(|registration| registration.data().domain == "es-fluent-lang")
            .expect("es-fluent-lang module registration should be present");
        let localizer = registration
            .create_localizer()
            .expect("es-fluent-lang should provide a localizer");
        localizer
            .select_language(&langid!("en-US"))
            .expect("language selection should succeed");

        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("English".to_string())
        );
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn force_link_reports_linked_resources() {
        assert!(force_link() > 0);
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
    fn localized_mode_returns_translated_language_names() {
        let module = EsFluentLanguageModule;
        let localizer = I18nModule::create_localizer(&module);

        localizer
            .select_language(&langid!("en"))
            .expect("English language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("French".to_string()),
            "With English UI, French should be 'French'"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-de", None),
            Some("German".to_string()),
            "With English UI, German should be 'German'"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-ja", None),
            Some("Japanese".to_string()),
            "With English UI, Japanese should be 'Japanese'"
        );

        localizer
            .select_language(&langid!("fr"))
            .expect("French language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("français".to_string()),
            "With French UI, French should be 'français'"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-de", None),
            Some("allemand".to_string()),
            "With French UI, German should be 'allemand'"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("anglais".to_string()),
            "With French UI, English should be 'anglais'"
        );

        localizer
            .select_language(&langid!("ja"))
            .expect("Japanese language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("フランス語".to_string()),
            "With Japanese UI, French should be 'フランス語'"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-de", None),
            Some("ドイツ語".to_string()),
            "With Japanese UI, German should be 'ドイツ語'"
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("英語".to_string()),
            "With Japanese UI, English should be '英語'"
        );
    }

    #[test]
    fn localized_mode_fallback_to_base_locale() {
        let module = EsFluentLanguageModule;
        let localizer = I18nModule::create_localizer(&module);

        localizer
            .select_language(&langid!("en-US"))
            .expect("en-US should fall back to en");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("French".to_string()),
            "en-US should fall back to en translations"
        );

        localizer
            .select_language(&langid!("fr-FR"))
            .expect("fr-FR should fall back to fr");
        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("anglais".to_string()),
            "fr-FR should fall back to fr translations"
        );
    }

    #[test]
    fn parse_embedded_resource_rejects_invalid_utf8() {
        let err = parse_embedded_resource("fr/es-fluent-lang.ftl", &[0xFF, 0xFE])
            .expect_err("invalid utf-8 should fail");
        assert!(matches!(err, LocalizationError::IoError(_)));
        assert!(err.to_string().contains("Invalid UTF-8"));
    }

    #[test]
    fn parse_embedded_resource_rejects_invalid_fluent() {
        let err = parse_embedded_resource("fr/es-fluent-lang.ftl", b"broken = {")
            .expect_err("invalid fluent should fail");
        assert!(matches!(err, LocalizationError::FluentParseError(_)));
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn bevy_uses_standard_module_registration_for_localized_names() {
        let registration = inventory::iter::<&'static dyn I18nModuleRegistration>()
            .find(|registration| registration.data().domain == "es-fluent-lang")
            .expect("es-fluent-lang module registration should be present");
        let localizer = registration
            .create_localizer()
            .expect("es-fluent-lang should provide a localizer");

        localizer
            .select_language(&langid!("fr"))
            .expect("language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("anglais".to_string())
        );
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn force_link_reports_linked_resources() {
        assert!(force_link() > 0);
    }
}
