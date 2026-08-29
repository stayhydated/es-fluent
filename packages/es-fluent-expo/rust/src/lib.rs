#![doc = include_str!("../README.md")]

use fluent_bundle::{FluentArgs, FluentResource, FluentValue, bundle::FluentBundle};
use fluent_langneg::{NegotiationStrategy, negotiate_languages};
use intl_memoizer::concurrent::IntlLangMemoizer;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

type NativeBundle = FluentBundle<Arc<FluentResource>, IntlLangMemoizer>;

#[derive(Clone, Debug, Eq, PartialEq, uniffi::Record)]
pub struct ExpoResource {
    pub path: String,
    pub source: String,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ExpoI18nError {
    #[error("invalid es-fluent manifest: {0}")]
    Manifest(String),
    #[error("invalid locale: {0}")]
    Locale(String),
    #[error("invalid Fluent resource: {0}")]
    Resource(String),
    #[error("invalid message arguments: {0}")]
    Arguments(String),
    #[error("missing Fluent message: {0}")]
    MissingMessage(String),
    #[error("failed to format Fluent message: {0}")]
    Format(String),
    #[error("invalid es-fluent snapshot: {0}")]
    Snapshot(String),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    revision: String,
    packages: Vec<ManifestPackage>,
    resources: Vec<ManifestResource>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestPackage {
    owner: String,
    fallback_locale: String,
    locales: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ManifestResource {
    locale: String,
    owner: String,
    domain: String,
    path: String,
}

#[derive(Clone, Debug)]
struct Package {
    fallback_locale: LanguageIdentifier,
    locales: Vec<LanguageIdentifier>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BundleKey {
    owner: String,
    locale: String,
    domain: String,
}

struct RuntimeInner {
    revision: String,
    package_order: Vec<String>,
    packages: BTreeMap<String, Package>,
    bundles: BTreeMap<BundleKey, NativeBundle>,
}

#[derive(uniffi::Object)]
pub struct ExpoI18nRuntime {
    inner: Arc<RuntimeInner>,
}

#[derive(uniffi::Object)]
pub struct ExpoI18nRequest {
    runtime: Arc<RuntimeInner>,
    requested_locales: Vec<String>,
    resolved_locales: BTreeMap<String, Vec<String>>,
    locale: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    schema_version: u32,
    revision: String,
    requested_locales: Vec<String>,
    resolved_locales: BTreeMap<String, Vec<String>>,
}

#[uniffi::export]
impl ExpoI18nRuntime {
    #[uniffi::constructor]
    pub fn new(
        manifest_json: String,
        resources: Vec<ExpoResource>,
        use_isolating: bool,
    ) -> Result<Arc<Self>, ExpoI18nError> {
        let inner = RuntimeInner::new(&manifest_json, resources, use_isolating)?;
        Ok(Arc::new(Self {
            inner: Arc::new(inner),
        }))
    }

    pub fn revision(&self) -> String {
        self.inner.revision.clone()
    }

    pub fn create_request(
        &self,
        requested_locales: Vec<String>,
    ) -> Result<Arc<ExpoI18nRequest>, ExpoI18nError> {
        ExpoI18nRequest::new(self.inner.clone(), requested_locales)
    }

    pub fn hydrate(&self, snapshot_json: String) -> Result<Arc<ExpoI18nRequest>, ExpoI18nError> {
        let snapshot: Snapshot = serde_json::from_str(&snapshot_json)
            .map_err(|error| ExpoI18nError::Snapshot(error.to_string()))?;
        if snapshot.schema_version != 1 {
            return Err(ExpoI18nError::Snapshot(format!(
                "unsupported schema {}",
                snapshot.schema_version
            )));
        }
        if snapshot.revision != self.inner.revision {
            return Err(ExpoI18nError::Snapshot(format!(
                "revision {} does not match runtime revision {}",
                snapshot.revision, self.inner.revision
            )));
        }

        let request = ExpoI18nRequest::new(self.inner.clone(), snapshot.requested_locales)?;
        if request.resolved_locales != snapshot.resolved_locales {
            return Err(ExpoI18nError::Snapshot(
                "resolved locale chains do not match the manifest".to_string(),
            ));
        }
        Ok(request)
    }
}

#[uniffi::export]
impl ExpoI18nRequest {
    pub fn locale(&self) -> String {
        self.locale.clone()
    }

    pub fn requested_locales(&self) -> Vec<String> {
        self.requested_locales.clone()
    }

    pub fn resolved_locales(&self, owner: String) -> Result<Vec<String>, ExpoI18nError> {
        self.resolved_locales
            .get(&owner)
            .cloned()
            .ok_or_else(|| ExpoI18nError::Manifest(format!("unknown exported package {owner}")))
    }

    pub fn format(
        &self,
        owner: String,
        domain: String,
        id: String,
        arguments_json: Option<String>,
    ) -> Result<String, ExpoI18nError> {
        self.try_format(owner.clone(), domain.clone(), id.clone(), arguments_json)?
            .ok_or_else(|| {
                let locales = self
                    .resolved_locales
                    .get(&owner)
                    .map(|locales| locales.join(", "))
                    .unwrap_or_default();
                ExpoI18nError::MissingMessage(format!("{owner}/{domain}/{id} in locales {locales}"))
            })
    }

    pub fn try_format(
        &self,
        owner: String,
        domain: String,
        id: String,
        arguments_json: Option<String>,
    ) -> Result<Option<String>, ExpoI18nError> {
        let locales = self
            .resolved_locales
            .get(&owner)
            .ok_or_else(|| ExpoI18nError::Manifest(format!("unknown exported package {owner}")))?;
        let arguments = parse_arguments(arguments_json.as_deref())?;

        for locale in locales {
            let key = BundleKey {
                owner: owner.clone(),
                locale: locale.clone(),
                domain: domain.clone(),
            };
            let Some(bundle) = self.runtime.bundles.get(&key) else {
                continue;
            };
            let Some(pattern) = bundle.get_message(&id).and_then(|message| message.value()) else {
                continue;
            };
            let mut errors = Vec::new();
            let formatted = bundle.format_pattern(pattern, arguments.as_ref(), &mut errors);
            if !errors.is_empty() {
                return Err(ExpoI18nError::Format(format!(
                    "{owner}/{domain}/{id} for {locale}: {errors:?}"
                )));
            }
            return Ok(Some(formatted.into_owned()));
        }

        Ok(None)
    }

    pub fn snapshot_json(&self) -> Result<String, ExpoI18nError> {
        serde_json::to_string(&Snapshot {
            schema_version: 1,
            revision: self.runtime.revision.clone(),
            requested_locales: self.requested_locales.clone(),
            resolved_locales: self.resolved_locales.clone(),
        })
        .map_err(|error| ExpoI18nError::Snapshot(error.to_string()))
    }
}

impl RuntimeInner {
    fn new(
        manifest_json: &str,
        resources: Vec<ExpoResource>,
        use_isolating: bool,
    ) -> Result<Self, ExpoI18nError> {
        let manifest: Manifest = serde_json::from_str(manifest_json)
            .map_err(|error| ExpoI18nError::Manifest(error.to_string()))?;
        if manifest.schema_version != 1 {
            return Err(ExpoI18nError::Manifest(format!(
                "unsupported schema {}",
                manifest.schema_version
            )));
        }
        if manifest.revision.is_empty() {
            return Err(ExpoI18nError::Manifest(
                "revision must not be empty".to_string(),
            ));
        }

        let package_order = manifest
            .packages
            .iter()
            .map(|package| package.owner.clone())
            .collect();
        let packages = parse_packages(&manifest.packages)?;
        let sources = index_sources(resources)?;
        let bundles = build_bundles(&manifest.resources, &packages, &sources, use_isolating)?;

        Ok(Self {
            revision: manifest.revision,
            package_order,
            packages,
            bundles,
        })
    }
}

impl ExpoI18nRequest {
    fn new(
        runtime: Arc<RuntimeInner>,
        requested_locales: Vec<String>,
    ) -> Result<Arc<Self>, ExpoI18nError> {
        let requested_locales = normalize_requested_locales(requested_locales)?;
        let requested = requested_locales
            .iter()
            .map(|locale| {
                locale
                    .parse::<LanguageIdentifier>()
                    .map_err(|error| ExpoI18nError::Locale(format!("{locale}: {error}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut resolved_locales = BTreeMap::new();
        for (owner, package) in &runtime.packages {
            let negotiated = negotiate_languages(
                &requested,
                &package.locales,
                Some(&package.fallback_locale),
                NegotiationStrategy::Filtering,
            );
            resolved_locales.insert(
                owner.clone(),
                negotiated
                    .into_iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            );
        }
        let locale = runtime
            .package_order
            .iter()
            .find_map(|owner| {
                resolved_locales
                    .get(owner)
                    .and_then(|locales| locales.first())
                    .cloned()
            })
            .or_else(|| requested_locales.first().cloned())
            .unwrap_or_else(|| "und".to_string());

        Ok(Arc::new(Self {
            runtime,
            requested_locales,
            resolved_locales,
            locale,
        }))
    }
}

fn parse_packages(
    manifest_packages: &[ManifestPackage],
) -> Result<BTreeMap<String, Package>, ExpoI18nError> {
    let mut packages = BTreeMap::new();
    for package in manifest_packages {
        if packages.contains_key(&package.owner) {
            return Err(ExpoI18nError::Manifest(format!(
                "duplicate exported package {}",
                package.owner
            )));
        }
        let locales = package
            .locales
            .iter()
            .map(|locale| parse_manifest_locale(locale, &package.owner))
            .collect::<Result<Vec<_>, _>>()?;
        let fallback_locale = parse_manifest_locale(&package.fallback_locale, &package.owner)?;
        if !locales.contains(&fallback_locale) {
            return Err(ExpoI18nError::Manifest(format!(
                "fallback locale {} is not exported for package {}",
                package.fallback_locale, package.owner
            )));
        }
        packages.insert(
            package.owner.clone(),
            Package {
                fallback_locale,
                locales,
            },
        );
    }
    Ok(packages)
}

fn parse_manifest_locale(locale: &str, owner: &str) -> Result<LanguageIdentifier, ExpoI18nError> {
    locale.parse::<LanguageIdentifier>().map_err(|error| {
        ExpoI18nError::Manifest(format!(
            "invalid locale {locale} for package {owner}: {error}"
        ))
    })
}

fn index_sources(resources: Vec<ExpoResource>) -> Result<BTreeMap<String, String>, ExpoI18nError> {
    let mut sources = BTreeMap::new();
    for resource in resources {
        if sources
            .insert(resource.path.clone(), resource.source)
            .is_some()
        {
            return Err(ExpoI18nError::Manifest(format!(
                "duplicate resource source {}",
                resource.path
            )));
        }
    }
    Ok(sources)
}

fn build_bundles(
    resources: &[ManifestResource],
    packages: &BTreeMap<String, Package>,
    sources: &BTreeMap<String, String>,
    use_isolating: bool,
) -> Result<BTreeMap<BundleKey, NativeBundle>, ExpoI18nError> {
    let mut paths = BTreeSet::new();
    let mut grouped = BTreeMap::<BundleKey, Vec<&ManifestResource>>::new();
    for resource in resources {
        let package = packages.get(&resource.owner).ok_or_else(|| {
            ExpoI18nError::Manifest(format!(
                "resource {} names unknown package {}",
                resource.path, resource.owner
            ))
        })?;
        let locale = parse_manifest_locale(&resource.locale, &resource.owner)?;
        if !package.locales.contains(&locale) {
            return Err(ExpoI18nError::Manifest(format!(
                "resource {} names unexported locale {}",
                resource.path, resource.locale
            )));
        }
        if !paths.insert(resource.path.clone()) {
            return Err(ExpoI18nError::Manifest(format!(
                "duplicate exported resource path {}",
                resource.path
            )));
        }
        if !sources.contains_key(&resource.path) {
            return Err(ExpoI18nError::Resource(format!(
                "missing source for {}",
                resource.path
            )));
        }
        grouped
            .entry(BundleKey {
                owner: resource.owner.clone(),
                locale: locale.to_string(),
                domain: resource.domain.clone(),
            })
            .or_default()
            .push(resource);
    }
    if let Some(path) = sources.keys().find(|path| !paths.contains(*path)) {
        return Err(ExpoI18nError::Resource(format!(
            "source {path} is not declared by the manifest"
        )));
    }

    let mut bundles = BTreeMap::new();
    for (key, mut specs) in grouped {
        specs.sort_by(|left, right| left.path.cmp(&right.path));
        let locale = key
            .locale
            .parse::<LanguageIdentifier>()
            .map_err(|error| ExpoI18nError::Locale(format!("{}: {error}", key.locale)))?;
        let mut bundle = FluentBundle::new_concurrent(vec![locale]);
        bundle.set_use_isolating(use_isolating);
        for spec in specs {
            let source = sources
                .get(&spec.path)
                .ok_or_else(|| ExpoI18nError::Resource(spec.path.clone()))?;
            let resource = FluentResource::try_new(source.clone()).map_err(|(_, errors)| {
                ExpoI18nError::Resource(format!("{}: {errors:?}", spec.path))
            })?;
            bundle
                .add_resource(Arc::new(resource))
                .map_err(|errors| ExpoI18nError::Resource(format!("{}: {errors:?}", spec.path)))?;
        }
        bundles.insert(key, bundle);
    }
    Ok(bundles)
}

fn normalize_requested_locales(
    requested_locales: Vec<String>,
) -> Result<Vec<String>, ExpoI18nError> {
    let mut unique = BTreeSet::new();
    let mut normalized = Vec::new();
    for locale in requested_locales {
        let locale = locale.trim();
        if !locale.is_empty() && unique.insert(locale.to_string()) {
            normalized.push(locale.to_string());
        }
    }
    if normalized.is_empty() {
        return Err(ExpoI18nError::Locale(
            "at least one requested locale is required".to_string(),
        ));
    }
    Ok(normalized)
}

fn parse_arguments(
    arguments_json: Option<&str>,
) -> Result<Option<FluentArgs<'static>>, ExpoI18nError> {
    let Some(arguments_json) = arguments_json else {
        return Ok(None);
    };
    let values: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(arguments_json)
            .map_err(|error| ExpoI18nError::Arguments(error.to_string()))?;
    let mut arguments = FluentArgs::new();
    for (name, value) in values {
        let value = match value {
            serde_json::Value::String(value) => FluentValue::from(value),
            serde_json::Value::Number(value) => {
                let number = value.as_f64().ok_or_else(|| {
                    ExpoI18nError::Arguments(format!("{name} is not a finite JSON number"))
                })?;
                FluentValue::from(number)
            },
            _ => {
                return Err(ExpoI18nError::Arguments(format!(
                    "{name} must be a string or number"
                )));
            },
        };
        arguments.set(name, value);
    }
    Ok(Some(arguments))
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
      "schemaVersion": 1,
      "revision": "native-fixture",
      "packages": [
        { "owner": "app", "fallbackLocale": "en-US", "locales": ["en-US", "fr"] },
        { "owner": "shared", "fallbackLocale": "en-US", "locales": ["en-US", "fr"] }
      ],
      "resources": [
        { "locale": "en-US", "owner": "app", "domain": "app", "path": "app/en.ftl" },
        { "locale": "fr", "owner": "app", "domain": "app", "path": "app/fr.ftl" },
        { "locale": "en-US", "owner": "shared", "domain": "app", "path": "shared/en.ftl" },
        { "locale": "fr", "owner": "shared", "domain": "app", "path": "shared/fr.ftl" }
      ]
    }"#;

    fn resources() -> Vec<ExpoResource> {
        [
            (
                "app/en.ftl",
                "title = Application\nwelcome = Welcome, { $name }!\nfallback = English only\nitems = { $count ->\n    [one] One item\n   *[other] { $count } items\n}",
            ),
            (
                "app/fr.ftl",
                "title = Application française\nwelcome = Bonjour, { $name } !",
            ),
            ("shared/en.ftl", "title = Shared library"),
            ("shared/fr.ftl", "title = Bibliothèque partagée"),
        ]
        .into_iter()
        .map(|(path, source)| ExpoResource {
            path: path.to_string(),
            source: source.to_string(),
        })
        .collect()
    }

    fn runtime() -> Arc<ExpoI18nRuntime> {
        ExpoI18nRuntime::new(MANIFEST.to_string(), resources(), false)
            .expect("fixture runtime should build")
    }

    #[test]
    fn localizes_scoped_messages_arguments_and_fallbacks() {
        let request = runtime()
            .create_request(vec!["fr-CA".to_string(), "fr".to_string()])
            .expect("fixture request should resolve");

        assert_eq!(request.locale(), "fr");
        assert_eq!(
            request
                .format(
                    "app".to_string(),
                    "app".to_string(),
                    "welcome".to_string(),
                    Some(r#"{"name":"Ada"}"#.to_string()),
                )
                .expect("welcome should format"),
            "Bonjour, Ada !"
        );
        assert_eq!(
            request
                .format(
                    "app".to_string(),
                    "app".to_string(),
                    "fallback".to_string(),
                    None,
                )
                .expect("fallback should format"),
            "English only"
        );
        assert_eq!(
            request
                .format(
                    "app".to_string(),
                    "app".to_string(),
                    "items".to_string(),
                    Some(r#"{"count":2}"#.to_string()),
                )
                .expect("item count should format"),
            "2 items"
        );
        assert_eq!(
            request
                .format(
                    "shared".to_string(),
                    "app".to_string(),
                    "title".to_string(),
                    None,
                )
                .expect("shared title should format"),
            "Bibliothèque partagée"
        );
    }

    #[test]
    fn keeps_request_locale_state_independent() {
        let runtime = runtime();
        let french = runtime
            .create_request(vec!["fr".to_string()])
            .expect("French request should resolve");
        let english = runtime
            .create_request(vec!["en-US".to_string()])
            .expect("English request should resolve");

        assert_eq!(french.locale(), "fr");
        assert_eq!(english.locale(), "en-US");
        assert_eq!(
            french.resolved_locales("app".to_string()).unwrap(),
            vec!["fr", "en-US"]
        );
        assert_eq!(
            english.resolved_locales("app".to_string()).unwrap(),
            vec!["en-US"]
        );
    }

    #[test]
    fn primary_locale_follows_exported_package_order() {
        let manifest = r#"{
          "schemaVersion": 1,
          "revision": "ordered-packages",
          "packages": [
            { "owner": "shared", "fallbackLocale": "en-US", "locales": ["en-US"] },
            { "owner": "app", "fallbackLocale": "en-US", "locales": ["en-US", "fr"] }
          ],
          "resources": [
            { "locale": "en-US", "owner": "app", "domain": "app", "path": "app/en.ftl" },
            { "locale": "fr", "owner": "app", "domain": "app", "path": "app/fr.ftl" },
            { "locale": "en-US", "owner": "shared", "domain": "app", "path": "shared/en.ftl" }
          ]
        }"#;
        let resources = resources()
            .into_iter()
            .filter(|resource| resource.path != "shared/fr.ftl")
            .collect();
        let runtime = ExpoI18nRuntime::new(manifest.to_string(), resources, false)
            .expect("fixture runtime should build");
        let request = runtime
            .create_request(vec!["fr".to_string()])
            .expect("fixture request should resolve");

        assert_eq!(request.locale(), "en-US");
        assert_eq!(
            request.resolved_locales("app".to_string()).unwrap(),
            vec!["fr", "en-US"]
        );
    }

    #[test]
    fn snapshots_round_trip_and_verify_revision_and_chains() {
        let runtime = runtime();
        let request = runtime
            .create_request(vec!["fr".to_string()])
            .expect("request should resolve");
        let snapshot = request.snapshot_json().expect("snapshot should encode");
        let hydrated = runtime.hydrate(snapshot).expect("snapshot should hydrate");
        assert_eq!(hydrated.locale(), "fr");

        let stale = r#"{
          "schemaVersion":1,
          "revision":"stale",
          "requestedLocales":["fr"],
          "resolvedLocales":{"app":["fr","en-US"],"shared":["fr","en-US"]}
        }"#;
        assert!(matches!(
            runtime.hydrate(stale.to_string()),
            Err(ExpoI18nError::Snapshot(_))
        ));
    }

    #[test]
    fn reports_manifest_resource_argument_and_message_errors() {
        assert!(matches!(
            ExpoI18nRuntime::new("{}".to_string(), Vec::new(), false),
            Err(ExpoI18nError::Manifest(_))
        ));

        let request = runtime()
            .create_request(vec!["en-US".to_string()])
            .expect("request should resolve");
        assert!(matches!(
            request.format(
                "app".to_string(),
                "app".to_string(),
                "missing".to_string(),
                None,
            ),
            Err(ExpoI18nError::MissingMessage(_))
        ));
        assert!(matches!(
            request.format(
                "app".to_string(),
                "app".to_string(),
                "welcome".to_string(),
                Some(r#"{"name":true}"#.to_string()),
            ),
            Err(ExpoI18nError::Arguments(_))
        ));
    }
}
