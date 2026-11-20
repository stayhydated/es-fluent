use super::resource::{ModuleResourceSpec, ResourceKey};
use fluent_bundle::FluentResource;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

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
            path: spec.locale_relative_path.to_string(),
            required: spec.required,
        }
    }

    /// Constructs an asset-pipeline load error for a resource spec.
    pub fn load(spec: &ModuleResourceSpec, details: impl Into<String>) -> Self {
        Self::Load {
            key: spec.key.clone(),
            path: spec.locale_relative_path.to_string(),
            required: spec.required,
            details: details.into(),
        }
    }

    /// Constructs a Fluent parse error for a resource spec.
    pub fn parse(spec: &ModuleResourceSpec, details: impl Into<String>) -> Self {
        Self::Parse {
            key: spec.key.clone(),
            path: spec.locale_relative_path.to_string(),
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

/// Loads a locale while retaining the resource key for domain-aware bundling.
pub fn load_locale_resource_entries(
    plan: &[ModuleResourceSpec],
    mut load: impl FnMut(&ModuleResourceSpec) -> ResourceLoadStatus,
) -> (Vec<(ResourceKey, Arc<FluentResource>)>, LocaleLoadReport) {
    let mut report = LocaleLoadReport::from_plan(plan);
    let mut resources = Vec::new();

    for spec in plan {
        match load(spec) {
            ResourceLoadStatus::Loaded(resource) => {
                report.mark_loaded(spec.key.clone());
                resources.push((spec.key.clone(), resource));
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

/// Parses UTF-8 bytes into a `FluentResource` using the shared load contract.
pub fn parse_fluent_resource_bytes(
    spec: &ModuleResourceSpec,
    bytes: &[u8],
) -> Result<Arc<FluentResource>, ResourceLoadError> {
    let content =
        String::from_utf8(bytes.to_vec()).map_err(|e| ResourceLoadError::InvalidUtf8 {
            key: spec.key.clone(),
            path: spec.locale_relative_path.to_string(),
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
        .map_err(|(_, errs)| ResourceLoadError::parse(spec, format!("{errs:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_localization::LocaleRelativeFtlPath;

    fn spec(key: &str, required: bool) -> ModuleResourceSpec {
        ModuleResourceSpec {
            key: ResourceKey::try_new(key).unwrap_or_else(|error| {
                panic!("test resource key '{key}' should be valid: {error}")
            }),
            locale_relative_path: LocaleRelativeFtlPath::try_new(format!("{key}.ftl"))
                .unwrap_or_else(|error| {
                    panic!("test FTL path '{key}.ftl' should be valid: {error}")
                }),
            required,
        }
    }

    #[test]
    fn load_locale_resources_records_loaded_missing_and_error_states() {
        let required = spec("app", true);
        let optional_missing = spec("app/optional", false);
        let optional_error = spec("app/broken", false);
        let plan = vec![required.clone(), optional_missing.clone(), optional_error];
        let loaded = Arc::new(
            FluentResource::try_new("hello = Hello".to_string()).expect("valid fluent resource"),
        );

        let (resources, report) = load_locale_resource_entries(&plan, |candidate| {
            if candidate.key == required.key {
                ResourceLoadStatus::Loaded(loaded.clone())
            } else if candidate.key == optional_missing.key {
                ResourceLoadStatus::Missing
            } else {
                ResourceLoadStatus::Error(ResourceLoadError::load(candidate, "failed"))
            }
        });

        assert_eq!(resources.len(), 1);
        assert!(report.required_keys().contains(&required.key));
        assert!(report.optional_keys().contains(&optional_missing.key));
        assert!(report.loaded_keys().contains(&required.key));
        assert_eq!(report.errors().len(), 2);
        assert!(report.is_ready());
    }

    #[test]
    fn locale_load_report_required_errors_make_locale_unready() {
        let required = spec("app", true);
        let mut report = LocaleLoadReport::from_plan(std::slice::from_ref(&required));

        report.mark_loaded(required.key.clone());
        assert!(report.is_ready());

        report.record_error(ResourceLoadError::missing(&required));
        assert!(report.has_required_errors());
        assert!(report.missing_required_keys().contains(&required.key));
        assert!(!report.is_ready());
    }

    #[test]
    fn resource_load_error_display_messages_cover_all_failure_kinds() {
        let required = spec("app", true);
        let optional = spec("optional", false);

        let missing = ResourceLoadError::missing(&optional);
        assert_eq!(
            missing.to_string(),
            "missing optional resource 'optional' at 'optional.ftl'"
        );
        assert!(!missing.is_required());

        let invalid_utf8 =
            parse_fluent_resource_bytes(&required, &[0xff]).expect_err("invalid UTF-8 should fail");
        assert!(
            invalid_utf8
                .to_string()
                .contains("invalid UTF-8 in required resource 'app'")
        );
        assert!(invalid_utf8.is_required());

        let parse = parse_fluent_resource_content(&optional, "hello = {".to_string())
            .expect_err("invalid Fluent should fail");
        assert!(
            parse
                .to_string()
                .contains("failed to parse optional resource 'optional'")
        );

        let load = ResourceLoadError::load(&required, "asset server failed");
        assert_eq!(
            load.to_string(),
            "failed to load required resource 'app' at 'app.ftl': asset server failed"
        );
        assert_eq!(load.key(), &required.key);
    }

    #[test]
    fn parse_fluent_resource_bytes_reports_invalid_utf8_with_resource_metadata() {
        let resource_spec = spec("app", true);
        let err = parse_fluent_resource_bytes(&resource_spec, &[0xff])
            .expect_err("invalid UTF-8 should be reported");

        assert!(matches!(
            err,
            ResourceLoadError::InvalidUtf8 {
                key,
                path,
                required: true,
                ..
            } if key == resource_spec.key && path == resource_spec.locale_relative_path.as_str()
        ));
    }
}
