#![doc = include_str!("../README.md")]

use es_fluent::{FluentLocalizer, FluentLocalizerExt, FluentMessage, FluentValue};
use es_fluent_manager_core::{FluentManager, ModuleDiscoveryError};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[doc(hidden)]
pub use es_fluent::__inventory;

#[doc(hidden)]
pub use es_fluent::__rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_embedded_i18n_module as define_i18n_module;

pub use es_fluent_manager_core::LocalizationError;

#[derive(Debug)]
pub enum EmbeddedInitError {
    ModuleDiscovery(Vec<ModuleDiscoveryError>),
    LanguageSelection(LocalizationError),
}

impl std::fmt::Display for EmbeddedInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleDiscovery(errors) => {
                f.write_str("failed strict i18n module discovery")?;
                for error in errors {
                    write!(f, "\n- {error}")?;
                }
                Ok(())
            },
            Self::LanguageSelection(error) => {
                write!(f, "failed to select the requested language: {error}")
            },
        }
    }
}

impl std::error::Error for EmbeddedInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::LanguageSelection(error) => Some(error),
        }
    }
}

/// Explicit embedded localization context.
///
/// Construct this once during application startup, keep it in application state,
/// and pass it to code that needs localization. No context-free `es-fluent`
/// context is installed.
#[derive(Clone)]
pub struct EmbeddedI18n {
    manager: Arc<FluentManager>,
}

impl EmbeddedI18n {
    fn from_manager(manager: FluentManager) -> Self {
        Self {
            manager: Arc::new(manager),
        }
    }

    /// Builds an embedded context without selecting a language.
    pub fn try_new() -> Result<Self, EmbeddedInitError> {
        FluentManager::try_new_with_discovered_modules()
            .map(Self::from_manager)
            .map_err(EmbeddedInitError::ModuleDiscovery)
    }

    /// Builds an embedded context and selects the initial active language.
    pub fn try_new_with_language<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, EmbeddedInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(EmbeddedInitError::ModuleDiscovery)?;
        manager
            .select_language(&lang)
            .map_err(EmbeddedInitError::LanguageSelection)?;
        Ok(Self::from_manager(manager))
    }

    /// Builds an embedded context and selects the initial active language,
    /// failing if any runtime module rejects the requested locale.
    pub fn try_new_with_language_strict<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, EmbeddedInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(EmbeddedInitError::ModuleDiscovery)?;
        manager
            .select_language_strict(&lang)
            .map_err(EmbeddedInitError::LanguageSelection)?;
        Ok(Self::from_manager(manager))
    }

    /// Selects the active language for this context.
    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.manager.select_language(&lang.into())
    }

    /// Selects the active language for this context and fails if any runtime
    /// module rejects the requested locale.
    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.manager.select_language_strict(&lang.into())
    }

    /// Renders a derived typed message through this context.
    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        FluentLocalizerExt::localize_message(self, message)
    }
}

impl FluentLocalizer for EmbeddedI18n {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize(id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize_in_domain(domain, id, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_manager_core::{
        I18nModule, I18nModuleDescriptor, I18nModuleRegistration, Localizer, ModuleData,
    };
    use std::sync::{Mutex, Once};
    use unic_langid::langid;

    static TEST_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en-US"), langid!("fr")];
    static TEST_MODULE_DATA: ModuleData = ModuleData {
        name: "embedded-test-module",
        domain: "embedded-test-module",
        supported_languages: TEST_SUPPORTED_LANGUAGES,
        namespaces: &[],
    };

    struct TestModule;

    struct TestLocalizer {
        selected: Mutex<LanguageIdentifier>,
    }

    impl I18nModuleDescriptor for TestModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_MODULE_DATA
        }
    }

    impl I18nModule for TestModule {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(TestLocalizer {
                selected: Mutex::new(langid!("en-US")),
            })
        }
    }

    impl Localizer for TestLocalizer {
        fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            if TEST_SUPPORTED_LANGUAGES
                .iter()
                .any(|candidate| candidate == lang)
            {
                let mut selected = self
                    .selected
                    .lock()
                    .expect("test localizer language lock should not be poisoned");
                *selected = lang.clone();
                Ok(())
            } else {
                Err(LocalizationError::LanguageNotSupported(lang.clone()))
            }
        }

        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            let selected = self
                .selected
                .lock()
                .expect("test localizer language lock should not be poisoned")
                .to_string();
            let value = match (selected.as_str(), id) {
                ("en-US", "hello") => "Hello",
                ("fr", "hello") => "Bonjour",
                _ => return None,
            };

            Some(value.to_string())
        }
    }

    struct TestMessage;

    impl FluentMessage for TestMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            localize("embedded-test-module", "hello", None)
        }
    }

    static TEST_MODULE: TestModule = TestModule;
    static INVENTORY_ONCE: Once = Once::new();

    crate::__inventory::submit!(&TEST_MODULE as &dyn I18nModuleRegistration);

    fn force_inventory_link() {
        INVENTORY_ONCE.call_once(|| {
            let _ = &TEST_MODULE;
        });
    }

    #[test]
    fn embedded_i18n_instances_select_languages_independently() {
        force_inventory_link();
        let en = EmbeddedI18n::try_new_with_language(langid!("en-US"))
            .expect("en embedded i18n should initialize");
        let fr = EmbeddedI18n::try_new_with_language(langid!("fr"))
            .expect("fr embedded i18n should initialize");

        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &en,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Hello".to_string())
        );
        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &fr,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Bonjour".to_string())
        );

        en.select_language(langid!("fr"))
            .expect("en manager should switch to fr");

        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &en,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Bonjour".to_string())
        );
        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &fr,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Bonjour".to_string())
        );

        fr.select_language(langid!("en-US"))
            .expect("fr manager should switch to en-US");

        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &en,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Bonjour".to_string())
        );
        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &fr,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Hello".to_string())
        );
    }

    #[test]
    fn embedded_i18n_facade_methods_delegate_to_manager_and_typed_lookup() {
        force_inventory_link();
        let i18n = EmbeddedI18n::try_new_with_language(langid!("en-US"))
            .expect("embedded i18n should initialize");

        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &i18n,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Hello".to_string())
        );
        assert_eq!(i18n.localize_message(&TestMessage), "Hello");
        assert!(
            i18n.select_language_strict(langid!("de")).is_err(),
            "strict selection should reject unsupported locales"
        );
    }

    #[test]
    fn embedded_i18n_try_new_builds_context_before_language_selection() {
        force_inventory_link();
        let i18n = EmbeddedI18n::try_new().expect("embedded i18n should initialize");
        let cloned = i18n.clone();

        assert_eq!(
            es_fluent::FluentLocalizer::localize(&i18n, "hello", None),
            None
        );
        cloned
            .select_language(langid!("fr"))
            .expect("language selection should work after initialization");
        assert_eq!(
            es_fluent::FluentLocalizer::localize_in_domain(
                &i18n,
                "embedded-test-module",
                "hello",
                None
            ),
            Some("Bonjour".to_string())
        );
    }

    #[test]
    fn embedded_init_error_display_and_source_match_error_kind() {
        use es_fluent_manager_core::{ModuleDiscoveryError, ModuleRegistrationKind};
        use std::error::Error;

        let discovery = EmbeddedInitError::ModuleDiscovery(vec![
            ModuleDiscoveryError::DuplicateModuleRegistration {
                name: "app".to_string(),
                domain: "app".to_string(),
                kind: ModuleRegistrationKind::MetadataOnly,
                count: 2,
            },
        ]);
        assert!(
            discovery
                .to_string()
                .contains("failed strict i18n module discovery")
        );
        assert!(discovery.source().is_none());

        let selection = EmbeddedInitError::LanguageSelection(
            LocalizationError::LanguageNotSupported(langid!("de")),
        );
        assert!(
            selection
                .to_string()
                .contains("failed to select the requested language")
        );
        assert!(selection.source().is_some());
    }
}
