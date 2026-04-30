#![doc = include_str!("../README.md")]

pub use unic_langid::{LanguageIdentifier, langid};

/// Force the linker to keep the `es-fluent-lang` runtime module.
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
    ModuleData,
};
use fluent_bundle::FluentValue;
use icu_experimental::displaynames::{DisplayNamesOptions, multi::LocaleDisplayNamesFormatter};
use icu_locale::Locale;
use parking_lot::RwLock;
use std::collections::HashMap;

const ES_FLUENT_LANG_PREFIX: &str = "es-fluent-lang-";
const DISPLAY_LANGUAGE_FALLBACKS: &[&str] = &["en", "en-001"];

fn parse_message_language(id: &str) -> Option<LanguageIdentifier> {
    id.strip_prefix(ES_FLUENT_LANG_PREFIX)?.parse().ok()
}

fn formatter_candidates(requested: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    let mut candidates = es_fluent_manager_core::locale_candidates(requested);

    for fallback in DISPLAY_LANGUAGE_FALLBACKS {
        if let Ok(language) = fallback.parse::<LanguageIdentifier>()
            && !candidates.iter().any(|candidate| candidate == &language)
        {
            candidates.push(language);
        }
    }

    candidates
}

fn format_language_name(
    display_language: &LanguageIdentifier,
    target_language: &LanguageIdentifier,
) -> Option<String> {
    let target_locale = target_language.to_string().parse::<Locale>().ok()?;

    for candidate in formatter_candidates(display_language) {
        let display_locale = match candidate.to_string().parse::<Locale>() {
            Ok(locale) => locale,
            Err(err) => {
                tracing::debug!(
                    "Skipping invalid ICU display locale candidate '{}': {}",
                    candidate,
                    err
                );
                continue;
            },
        };

        match LocaleDisplayNamesFormatter::try_new(
            display_locale.into(),
            DisplayNamesOptions::default(),
        ) {
            Ok(formatter) => return Some(formatter.of(&target_locale).into_owned()),
            Err(err) => tracing::debug!(
                "ICU display names formatter not available for '{}': {}",
                candidate,
                err
            ),
        }
    }

    None
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
        Box::new(EsFluentLanguageLocalizer::new(langid!("en-US")))
    }

    fn contributes_to_language_selection(&self) -> bool {
        false
    }
}

#[doc(hidden)]
struct EsFluentLanguageLocalizer {
    current_lang: RwLock<LanguageIdentifier>,
}

#[doc(hidden)]
impl EsFluentLanguageLocalizer {
    fn new(default_lang: LanguageIdentifier) -> Self {
        Self {
            current_lang: RwLock::new(default_lang),
        }
    }
}

#[doc(hidden)]
impl Localizer for EsFluentLanguageLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        *self.current_lang.write() = lang.clone();
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if args.is_some_and(|args| !args.is_empty()) {
            tracing::debug!(
                "Ignoring Fluent args for built-in language label '{}'; ICU-backed labels do not accept arguments",
                id
            );
        }

        let target_language = parse_message_language(id)?;

        #[cfg(feature = "localized-langs")]
        let display_language = self.current_lang.read().clone();

        #[cfg(not(feature = "localized-langs"))]
        let display_language = target_language.clone();

        format_language_name(&display_language, &target_language)
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

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    #[test]
    fn parse_message_language_extracts_language_identifier() {
        assert_eq!(
            parse_message_language("es-fluent-lang-fr-FR"),
            Some(langid!("fr-FR"))
        );
        assert_eq!(parse_message_language("missing-prefix"), None);
        assert_eq!(parse_message_language("es-fluent-lang-invalid!"), None);
    }

    #[test]
    fn formatter_candidates_include_requested_chain_and_defaults() {
        let candidates = formatter_candidates(&langid!("fr-CA"));

        assert!(candidates.contains(&langid!("fr-CA")));
        assert!(candidates.contains(&langid!("fr")));
        assert!(candidates.contains(&langid!("en")));
        assert!(candidates.contains(&langid!("en-001")));
    }

    #[test]
    fn language_module_creates_localizer_and_selects_language() {
        let module = EsFluentLanguageModule;
        assert_eq!(module.data().name, "es-fluent-lang");

        let localizer = I18nModule::create_localizer(&module);
        localizer
            .select_language(&langid!("en-US"))
            .expect("language selection should succeed");

        #[cfg(not(feature = "localized-langs"))]
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some(expected_french_name())
        );

        #[cfg(feature = "localized-langs")]
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("French".to_string())
        );
    }

    #[cfg(not(feature = "localized-langs"))]
    #[test]
    fn autonym_mode_returns_native_language_names_regardless_of_selected_locale() {
        let module = EsFluentLanguageModule;
        let localizer = I18nModule::create_localizer(&module);

        localizer
            .select_language(&langid!("en-US"))
            .expect("language selection should succeed");

        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("English".to_string())
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some(expected_french_name())
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-ja", None),
            Some(expected_japanese_name())
        );

        localizer
            .select_language(&langid!("ja-JP"))
            .expect("language selection should succeed");

        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some(expected_french_name())
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-ja", None),
            Some(expected_japanese_name())
        );
    }

    #[cfg(feature = "localized-langs")]
    #[test]
    fn localized_mode_returns_translated_language_names() {
        let module = EsFluentLanguageModule;
        let localizer = I18nModule::create_localizer(&module);

        localizer
            .select_language(&langid!("en"))
            .expect("English language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some("French".to_string())
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-de", None),
            Some("German".to_string())
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-ja", None),
            Some("Japanese".to_string())
        );

        localizer
            .select_language(&langid!("fr"))
            .expect("French language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", None),
            Some(expected_french_name())
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-de", None),
            Some("allemand".to_string())
        );
        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("anglais".to_string())
        );
    }

    #[cfg(feature = "localized-langs")]
    #[test]
    fn localized_mode_falls_back_to_parent_display_locale() {
        let module = EsFluentLanguageModule;
        let localizer = I18nModule::create_localizer(&module);

        localizer
            .select_language(&langid!("fr-FR"))
            .expect("fr-FR should fall back to fr");
        assert_eq!(
            localizer.localize("es-fluent-lang-en", None),
            Some("anglais".to_string())
        );
    }

    #[cfg(feature = "localized-langs")]
    #[test]
    fn localized_mode_ignores_unused_args() {
        let module = EsFluentLanguageModule;
        let localizer = I18nModule::create_localizer(&module);
        let mut args = HashMap::new();
        args.insert("unused", FluentValue::from("value"));

        localizer
            .select_language(&langid!("en"))
            .expect("language selection should succeed");
        assert_eq!(
            localizer.localize("es-fluent-lang-fr", Some(&args)),
            Some("French".to_string())
        );
    }

    #[cfg(feature = "bevy")]
    #[test]
    fn bevy_uses_standard_module_registration() {
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

    fn expected_french_name() -> String {
        "français".to_string()
    }

    #[cfg(not(feature = "localized-langs"))]
    fn expected_japanese_name() -> String {
        "日本語".to_string()
    }
}
