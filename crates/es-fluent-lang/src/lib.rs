#![doc = include_str!("../README.md")]

pub use unic_langid::{LanguageIdentifier, langid};

/// Force the linker to keep the `es-fluent-lang` runtime module.
///
/// This is used by WASM examples where localization inventory registration can be
/// stripped by aggressive release optimization.
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
#[used]
static FORCE_LINK_KEEPALIVE: fn() -> usize = force_link_support::force_link;

#[doc(hidden)]
#[inline(never)]
pub fn force_link() -> usize {
    force_link_support::force_link()
}

#[cfg(feature = "macros")]
pub use es_fluent_lang_macro::es_fluent_language;

#[doc(hidden)]
use es_fluent_manager_core::{
    FluentArgumentMap, I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError,
    Localizer, ModuleData, ModuleDomain, StaticFluentMessageKey,
};
use icu_experimental::displaynames::{DisplayNamesOptions, multi::LocaleDisplayNamesFormatter};
use icu_locale::Locale;

const ES_FLUENT_LANG_PREFIX: &str = "es-fluent-lang-";
const DISPLAY_LANGUAGE_FALLBACKS: &[&str] = &["en", "en-001"];
#[doc(hidden)]
pub const WASM_FORCE_LINK_MARKER: &str = "es-fluent-lang-wasm-force-link";

#[cfg(target_arch = "wasm32")]
#[used]
static WASM_FORCE_LINK_MARKER_BYTES: [u8; WASM_FORCE_LINK_MARKER.len()] =
    *b"es-fluent-lang-wasm-force-link";

fn parse_message_language(id: &str) -> Option<LanguageIdentifier> {
    id.strip_prefix(ES_FLUENT_LANG_PREFIX)
        .unwrap_or(id)
        .parse()
        .ok()
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

/// Builds a display-names formatter for the first usable locale in
/// `display_language`'s fallback chain.
fn display_names_formatter(
    display_language: &LanguageIdentifier,
) -> Option<LocaleDisplayNamesFormatter> {
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
            Ok(formatter) => return Some(formatter),
            Err(err) => tracing::debug!(
                "ICU display names formatter not available for '{}': {}",
                candidate,
                err
            ),
        }
    }

    None
}

fn format_language_name(
    formatter: &LocaleDisplayNamesFormatter,
    target_language: &LanguageIdentifier,
) -> Option<String> {
    let target_locale = target_language.to_string().parse::<Locale>().ok()?;
    Some(formatter.of(&target_locale).into_owned())
}

/// Returns the label naming `target_language` in that language.
///
/// The label does not depend on the selected locale, so it is formatted once
/// per process and later lookups skip building an ICU formatter.
#[cfg(not(feature = "localized-langs"))]
fn autonym(target_language: &LanguageIdentifier) -> Option<String> {
    use std::{
        collections::HashMap,
        sync::{LazyLock, PoisonError, RwLock},
    };

    static AUTONYMS: LazyLock<RwLock<HashMap<LanguageIdentifier, Option<String>>>> =
        LazyLock::new(RwLock::default);

    if let Some(label) = AUTONYMS
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(target_language)
    {
        return label.clone();
    }

    let label = display_names_formatter(target_language)
        .and_then(|formatter| format_language_name(&formatter, target_language));
    AUTONYMS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(target_language.clone())
        .or_insert(label)
        .clone()
}

#[doc(hidden)]
struct EsFluentLanguageModule;

static ES_FLUENT_LANG_MODULE_DATA: ModuleData = ModuleData {
    name: "es-fluent-lang",
    owner: es_fluent_manager_core::__macro::static_domain("es-fluent-lang"),
    supported_languages: &[],
    domains: &[ModuleDomain {
        domain: es_fluent_manager_core::__macro::static_domain("es-fluent-lang"),
        namespaces: &[],
    }],
};

impl I18nModuleDescriptor for EsFluentLanguageModule {
    fn data(&self) -> &'static ModuleData {
        &ES_FLUENT_LANG_MODULE_DATA
    }
}

impl I18nModule for EsFluentLanguageModule {
    fn create_localizer(
        &self,
        lang: &LanguageIdentifier,
    ) -> Result<Box<dyn Localizer>, LocalizationError> {
        Ok(Box::new(EsFluentLanguageLocalizer::new(lang)))
    }

    fn contributes_to_language_selection(&self) -> bool {
        false
    }
}

/// Formats language labels for one selected locale.
#[doc(hidden)]
struct EsFluentLanguageLocalizer {
    /// Formatter for labels in the selected locale, built once per selection.
    #[cfg(feature = "localized-langs")]
    formatter: Option<LocaleDisplayNamesFormatter>,
}

#[doc(hidden)]
impl EsFluentLanguageLocalizer {
    #[cfg_attr(not(feature = "localized-langs"), allow(unused_variables))]
    fn new(lang: &LanguageIdentifier) -> Self {
        Self {
            #[cfg(feature = "localized-langs")]
            formatter: display_names_formatter(lang),
        }
    }
}

#[doc(hidden)]
impl Localizer for EsFluentLanguageLocalizer {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgumentMap<'a>>,
    ) -> Option<String> {
        if key.owner() != ES_FLUENT_LANG_MODULE_DATA.owner
            || !ES_FLUENT_LANG_MODULE_DATA.owns_domain(key.domain())
        {
            return None;
        }
        if args.is_some_and(|args| args.iter().next().is_some()) {
            tracing::debug!(
                "Ignoring Fluent args for built-in language label '{}'; ICU-backed labels do not accept arguments",
                key.id().as_str()
            );
        }

        let target_language = parse_message_language(key.id().as_str())?;

        // Each label names its language in the selected locale.
        #[cfg(feature = "localized-langs")]
        return format_language_name(self.formatter.as_ref()?, &target_language);

        // Each label names its language in that language.
        #[cfg(not(feature = "localized-langs"))]
        autonym(&target_language)
    }
}

inventory::submit! {
    &ES_FLUENT_LANGUAGE_MODULE as &dyn I18nModuleRegistration
}

static ES_FLUENT_LANGUAGE_MODULE: EsFluentLanguageModule = EsFluentLanguageModule;

#[doc(hidden)]
mod force_link_support {
    use super::*;

    pub(crate) fn force_link() -> usize {
        let module: &'static dyn I18nModuleRegistration = &ES_FLUENT_LANGUAGE_MODULE;
        let _ = module.create_localizer(&langid!("en-US"));
        usize::from(!module.data().owner.as_str().is_empty())
    }
}

#[cfg(test)]
mod tests;
