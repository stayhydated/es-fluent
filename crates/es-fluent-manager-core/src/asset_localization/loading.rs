use super::resource::{ModuleResourceSpec, ResourceKey};
use fluent_bundle::FluentResource;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// Structured locale loading state shared across managers.
#[derive(Clone, Debug, Default)]
pub struct LocaleLoadReport {
    required_keys: HashSet<ResourceKey>,
    optional_keys: HashSet<ResourceKey>,
    loaded_keys: HashSet<ResourceKey>,
    errors: Vec<ResourceLoadError>,
}

impl LocaleLoadReport {
    /// Builds a new report from a canonical resource plan.
    pub fn from_plan(plan: &[ModuleResourceSpec]) -> Self {
        Self::from_specs(plan.iter())
    }

    /// Builds a new report from resource specs.
    pub fn from_specs<'a>(specs: impl IntoIterator<Item = &'a ModuleResourceSpec>) -> Self {
        let mut required_keys = HashSet::new();
        let mut optional_keys = HashSet::new();

        for spec in specs {
            if spec.required {
                required_keys.insert(spec.key.clone());
            } else {
                optional_keys.insert(spec.key.clone());
            }
        }

        Self {
            required_keys,
            optional_keys,
            loaded_keys: HashSet::new(),
            errors: Vec::new(),
        }
    }

    /// Marks a resource key as loaded.
    pub fn mark_loaded(&mut self, key: ResourceKey) {
        self.loaded_keys.insert(key);
    }

    /// Records a resource load error and removes the corresponding loaded key.
    pub fn record_error(&mut self, error: ResourceLoadError) {
        self.loaded_keys.remove(error.key());
        self.errors.push(error);
    }

    /// Returns required keys from the report.
    pub fn required_keys(&self) -> &HashSet<ResourceKey> {
        &self.required_keys
    }

    /// Returns optional keys from the report.
    pub fn optional_keys(&self) -> &HashSet<ResourceKey> {
        &self.optional_keys
    }

    /// Returns loaded keys from the report.
    pub fn loaded_keys(&self) -> &HashSet<ResourceKey> {
        &self.loaded_keys
    }

    /// Returns all recorded load errors.
    pub fn errors(&self) -> &[ResourceLoadError] {
        &self.errors
    }

    /// Returns required keys that are still missing.
    pub fn missing_required_keys(&self) -> HashSet<ResourceKey> {
        self.required_keys
            .iter()
            .filter(|key| !self.loaded_keys.contains(*key))
            .cloned()
            .collect()
    }

    /// Returns true when a required resource failed loading.
    pub fn has_required_errors(&self) -> bool {
        self.errors.iter().any(ResourceLoadError::is_required)
    }

    /// Returns true when locale readiness requirements are met.
    pub fn is_ready(&self) -> bool {
        super::resource::locale_is_ready(&self.required_keys, &self.loaded_keys)
            && !self.has_required_errors()
    }
}

/// Canonical resource-load failure categories shared across managers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceLoadError {
    /// A required resource was not present.
    Missing {
        key: ResourceKey,
        path: String,
        required: bool,
    },
    /// Resource bytes were not valid UTF-8.
    InvalidUtf8 {
        key: ResourceKey,
        path: String,
        required: bool,
        details: String,
    },
    /// Resource content failed Fluent parsing.
    Parse {
        key: ResourceKey,
        path: String,
        required: bool,
        details: String,
    },
    /// Resource loading failed in the host asset pipeline.
    Load {
        key: ResourceKey,
        path: String,
        required: bool,
        details: String,
    },
}

impl ResourceLoadError {
    /// Constructs a missing-file error for a resource spec.
    pub fn missing(spec: &ModuleResourceSpec) -> Self {
        Self::Missing {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
        }
    }

    /// Constructs an asset-pipeline load error for a resource spec.
    pub fn load(spec: &ModuleResourceSpec, details: impl Into<String>) -> Self {
        Self::Load {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
            details: details.into(),
        }
    }

    /// Returns the key associated with this failure.
    pub fn key(&self) -> &ResourceKey {
        match self {
            Self::Missing { key, .. }
            | Self::InvalidUtf8 { key, .. }
            | Self::Parse { key, .. }
            | Self::Load { key, .. } => key,
        }
    }

    /// Returns true when this failure affects required readiness.
    pub fn is_required(&self) -> bool {
        match self {
            Self::Missing { required, .. }
            | Self::InvalidUtf8 { required, .. }
            | Self::Parse { required, .. }
            | Self::Load { required, .. } => *required,
        }
    }
}

impl fmt::Display for ResourceLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing {
                key,
                path,
                required,
            } => write!(
                f,
                "missing {} resource '{}' at '{}'",
                if *required { "required" } else { "optional" },
                key,
                path
            ),
            Self::InvalidUtf8 {
                key,
                path,
                required,
                details,
            } => write!(
                f,
                "invalid UTF-8 in {} resource '{}' at '{}': {}",
                if *required { "required" } else { "optional" },
                key,
                path,
                details
            ),
            Self::Parse {
                key,
                path,
                required,
                details,
            } => write!(
                f,
                "failed to parse {} resource '{}' at '{}': {}",
                if *required { "required" } else { "optional" },
                key,
                path,
                details
            ),
            Self::Load {
                key,
                path,
                required,
                details,
            } => write!(
                f,
                "failed to load {} resource '{}' at '{}': {}",
                if *required { "required" } else { "optional" },
                key,
                path,
                details
            ),
        }
    }
}

impl std::error::Error for ResourceLoadError {}

/// Shared outcome type for loading one localized resource.
#[derive(Clone, Debug)]
pub enum ResourceLoadStatus {
    Loaded(Arc<FluentResource>),
    Missing,
    Error(ResourceLoadError),
}

/// Loads a locale from a resource plan while keeping readiness/error bookkeeping centralized.
pub fn load_locale_resources(
    plan: &[ModuleResourceSpec],
    mut load: impl FnMut(&ModuleResourceSpec) -> ResourceLoadStatus,
) -> (Vec<Arc<FluentResource>>, LocaleLoadReport) {
    let mut report = LocaleLoadReport::from_plan(plan);
    let mut resources = Vec::new();

    for spec in plan {
        match load(spec) {
            ResourceLoadStatus::Loaded(resource) => {
                report.mark_loaded(spec.key.clone());
                resources.push(resource);
            },
            ResourceLoadStatus::Missing => {
                report.record_error(ResourceLoadError::missing(spec));
            },
            ResourceLoadStatus::Error(error) => {
                report.record_error(error);
            },
        }
    }

    (resources, report)
}

/// Builds a detailed per-language load report from persistent localized resource state.
pub fn build_locale_load_report(
    resource_specs: &HashMap<(LanguageIdentifier, ResourceKey), ModuleResourceSpec>,
    loaded_resources: &HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    load_errors: &HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    lang: &LanguageIdentifier,
) -> LocaleLoadReport {
    let specs = resource_specs
        .iter()
        .filter_map(|((language, _), spec)| if language == lang { Some(spec) } else { None })
        .collect::<Vec<_>>();
    let mut report = LocaleLoadReport::from_specs(specs.iter().copied());

    for (language_key, resource_key) in loaded_resources.keys() {
        if language_key == lang {
            report.mark_loaded(resource_key.clone());
        }
    }

    for ((language_key, _), load_error) in load_errors {
        if language_key == lang {
            report.record_error(load_error.clone());
        }
    }

    report
}

/// Collects loaded `FluentResource`s for a locale in stable key order.
pub fn collect_locale_resources<'a>(
    loaded_resources: &'a HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    lang: &LanguageIdentifier,
) -> Vec<&'a Arc<FluentResource>> {
    let mut resources = loaded_resources
        .iter()
        .filter_map(|((language_key, resource_key), resource)| {
            if language_key == lang {
                Some((resource_key, resource))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    resources.sort_by_key(|(resource_key, _)| *resource_key);
    resources
        .into_iter()
        .map(|(_, resource)| resource)
        .collect()
}

/// Collects available languages from localized resource registrations.
pub fn collect_available_languages<V>(
    resources: &HashMap<(LanguageIdentifier, ResourceKey), V>,
) -> Vec<LanguageIdentifier> {
    let mut seen = HashSet::new();
    let mut languages = Vec::new();

    for (lang, _) in resources.keys() {
        if seen.insert(lang.clone()) {
            languages.push(lang.clone());
        }
    }

    languages.sort_by_key(|lang| lang.to_string());
    languages
}

/// Stores a successfully parsed localized resource and clears any prior error for the same key.
pub fn store_locale_resource(
    loaded_resources: &mut HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    load_errors: &mut HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    lang: &LanguageIdentifier,
    spec: &ModuleResourceSpec,
    resource: Arc<FluentResource>,
) {
    let key = (lang.clone(), spec.key.clone());
    loaded_resources.insert(key.clone(), resource);
    load_errors.remove(&key);
}

/// Parses source text and stores the resulting localized resource.
pub fn parse_and_store_locale_resource_content(
    loaded_resources: &mut HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    load_errors: &mut HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    lang: &LanguageIdentifier,
    spec: &ModuleResourceSpec,
    content: String,
) -> Result<(), ResourceLoadError> {
    let resource = parse_fluent_resource_content(spec, content)?;
    store_locale_resource(loaded_resources, load_errors, lang, spec, resource);
    Ok(())
}

/// Records a localized resource error and clears any previously loaded resource for the same key.
pub fn record_locale_resource_error(
    loaded_resources: &mut HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    load_errors: &mut HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    lang: &LanguageIdentifier,
    error: ResourceLoadError,
) {
    let key = (lang.clone(), error.key().clone());
    loaded_resources.remove(&key);
    load_errors.insert(key, error);
}

/// Records a missing localized resource.
pub fn record_missing_locale_resource(
    loaded_resources: &mut HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    load_errors: &mut HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    lang: &LanguageIdentifier,
    spec: &ModuleResourceSpec,
) -> ResourceLoadError {
    let error = ResourceLoadError::missing(spec);
    record_locale_resource_error(loaded_resources, load_errors, lang, error.clone());
    error
}

/// Records an asset-pipeline load failure for a localized resource.
pub fn record_failed_locale_resource(
    loaded_resources: &mut HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    load_errors: &mut HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    lang: &LanguageIdentifier,
    spec: &ModuleResourceSpec,
    details: impl Into<String>,
) -> ResourceLoadError {
    let error = ResourceLoadError::load(spec, details);
    record_locale_resource_error(loaded_resources, load_errors, lang, error.clone());
    error
}

/// Clears all tracked localized state for a resource key.
pub fn clear_locale_resource(
    loaded_resources: &mut HashMap<(LanguageIdentifier, ResourceKey), Arc<FluentResource>>,
    load_errors: &mut HashMap<(LanguageIdentifier, ResourceKey), ResourceLoadError>,
    lang: &LanguageIdentifier,
    key: &ResourceKey,
) {
    let state_key = (lang.clone(), key.clone());
    loaded_resources.remove(&state_key);
    load_errors.remove(&state_key);
}

/// Parses UTF-8 bytes into a `FluentResource` using the shared load contract.
pub fn parse_fluent_resource_bytes(
    spec: &ModuleResourceSpec,
    bytes: &[u8],
) -> Result<Arc<FluentResource>, ResourceLoadError> {
    let content =
        String::from_utf8(bytes.to_vec()).map_err(|e| ResourceLoadError::InvalidUtf8 {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
            details: e.to_string(),
        })?;

    parse_fluent_resource_content(spec, content)
}

/// Parses Fluent source text into a `FluentResource` using the shared load contract.
pub fn parse_fluent_resource_content(
    spec: &ModuleResourceSpec,
    content: String,
) -> Result<Arc<FluentResource>, ResourceLoadError> {
    FluentResource::try_new(content)
        .map(Arc::new)
        .map_err(|(_, errs)| ResourceLoadError::Parse {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
            details: format!("{errs:?}"),
        })
}

#[cfg(test)]
mod tests;
