use crate::localization::{I18nModule, Localizer, LocalizationError};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

/// Data for a statically-defined i18n module.
///
/// This struct is intended to be created by a macro at compile time.
#[derive(Debug)]
pub struct StaticModuleData {
    /// The name of the module.
    pub name: &'static str,
    /// An array of language identifiers and their corresponding FTL resource strings.
    /// The `LanguageIdentifier`s should be created using `unic_langid::langid!`.
    pub resources: &'static [(LanguageIdentifier, &'static str)],
}

/// A `Localizer` implementation that uses statically-defined resources.
#[derive(Debug)]
pub struct StaticLocalizer {
    data: &'static StaticModuleData,
    current_resource: RwLock<Option<Arc<FluentResource>>>,
    current_lang: RwLock<Option<LanguageIdentifier>>,
}

impl StaticLocalizer {
    pub fn new(data: &'static StaticModuleData) -> Self {
        Self {
            data,
            current_resource: RwLock::new(None),
            current_lang: RwLock::new(None),
        }
    }
}

impl Localizer for StaticLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        let mut current_lang_guard = self.current_lang.write().unwrap();
        if Some(lang) == current_lang_guard.as_ref() {
            return Ok(());
        }

        for (resource_lang, resource_content) in self.data.resources {
            if lang.matches(resource_lang, true, true) {
                let resource = FluentResource::try_new((*resource_content).to_string())
                    .map_err(|(_, errs)| {
                        LocalizationError::BackendError(anyhow::anyhow!(
                            "Failed to parse fluent resource: {:?}",
                            errs
                        ))
                    })?;

                *self.current_resource.write().unwrap() = Some(Arc::new(resource));
                *current_lang_guard = Some(lang.clone());
                return Ok(());
            }
        }
        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let resource_arc = self.current_resource.read().unwrap();
        if let Some(resource) = resource_arc.as_ref() {
            let lang_guard = self.current_lang.read().unwrap();
            let lang = lang_guard
                .as_ref()
                .expect("Language not selected before localization");

            let mut bundle = FluentBundle::new(vec![lang.clone()]);
            bundle
                .add_resource(resource.clone())
                .expect("Failed to add resource");

            if let Some(message) = bundle.get_message(id) {
                if let Some(pattern) = message.value() {
                    let mut errors = Vec::new();
                    let fluent_args = args.map(|args| {
                        let mut fa = FluentArgs::new();
                        for (key, value) in args {
                            fa.set(*key, value.clone());
                        }
                        fa
                    });
                    let value = bundle.format_pattern(pattern, fluent_args.as_ref(), &mut errors);
                    if errors.is_empty() {
                        return Some(value.into_owned());
                    } else {
                        log::error!("Fluent formatting errors: {:?}", errors);
                    }
                }
            }
        }
        None
    }
}

/// An `I18nModule` implementation that uses statically-defined resources.
///
/// This struct is intended to be created by a macro at compile time.
pub struct StaticI18nModule {
    data: &'static StaticModuleData,
}

impl StaticI18nModule {
    pub const fn new(data: &'static StaticModuleData) -> Self {
        Self { data }
    }
}

impl I18nModule for StaticI18nModule {
    fn name(&self) -> &'static str {
        self.data.name
    }

    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(StaticLocalizer::new(self.data))
    }
}
