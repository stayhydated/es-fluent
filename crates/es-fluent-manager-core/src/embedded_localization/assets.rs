use crate::asset_localization::{
    LocaleRelativeFtlPath, ModuleResourceSpec, ResourceKey, ResourceLoadError,
    parse_fluent_resource_bytes,
};
use fluent_bundle::FluentResource;
use std::sync::{Arc, OnceLock};
use unic_langid::LanguageIdentifier;

/// A locale resource whose contents were captured at compile time.
///
/// The contents are parsed on first use and the result is kept for the rest
/// of the process, so later language switches only assemble bundles.
#[derive(Debug)]
pub struct EmbeddedResource {
    key: &'static str,
    locale_relative_path: &'static str,
    required: bool,
    contents: &'static [u8],
    parsed: OnceLock<Result<Arc<FluentResource>, ResourceLoadError>>,
}

impl EmbeddedResource {
    /// Creates an embedded resource.
    ///
    /// `key` is the resource key, such as `my-crate/ui`, and
    /// `locale_relative_path` its path under the locale directory, such as
    /// `my-crate/ui.ftl`. A locale is unready without its `required` resources.
    pub const fn new(
        key: &'static str,
        locale_relative_path: &'static str,
        required: bool,
        contents: &'static [u8],
    ) -> Self {
        Self {
            key,
            locale_relative_path,
            required,
            contents,
            parsed: OnceLock::new(),
        }
    }

    /// Returns the resource plan entry for this resource.
    pub fn spec(&self) -> ModuleResourceSpec {
        ModuleResourceSpec::new(
            ResourceKey::from_static_path(self.key),
            LocaleRelativeFtlPath::from_static_path(self.locale_relative_path),
            self.required,
        )
    }

    /// Returns the parsed resource, parsing it on first use.
    pub fn resource(&self) -> Result<Arc<FluentResource>, ResourceLoadError> {
        self.parsed
            .get_or_init(|| parse_fluent_resource_bytes(&self.spec(), self.contents))
            .clone()
    }
}

/// The embedded resources of one locale.
///
/// `define_i18n_module!` plans each locale at build time and embeds every
/// planned file, so the binary does not depend on the location of the source
/// checkout that compiled it.
#[derive(Debug)]
pub struct EmbeddedLanguage {
    /// The locale.
    pub language: LanguageIdentifier,
    /// Every planned resource of the locale.
    pub resources: &'static [EmbeddedResource],
}

impl EmbeddedLanguage {
    /// Returns the resource plan of this locale.
    pub fn resource_plan(&self) -> Vec<ModuleResourceSpec> {
        self.resources.iter().map(EmbeddedResource::spec).collect()
    }
}
