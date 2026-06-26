#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), deny(clippy::panic, clippy::unwrap_used))]

use es_fluent_shared::{
    fluent::{FluentArgumentName, FluentEntryId},
    resource::ModuleResourceSpec,
    source::{SourceFile, SourceLine},
};
use fs_err as fs;
use std::path::{Path, PathBuf};

mod error;

pub use error::RunnerIoError;
pub use es_fluent_shared::FluentParseMode;

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct RunnerResult {
    pub changed: bool,
}

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct ExpectedKey {
    pub key: FluentEntryId,
    pub variables: Vec<FluentArgumentName>,
    #[serde(default)]
    pub resource: Option<ModuleResourceSpec>,
    pub source_file: Option<SourceFile>,
    pub source_line: Option<SourceLine>,
}

#[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct InventoryData {
    pub expected_keys: Vec<ExpectedKey>,
}

#[derive(Clone, Debug, derive_more::AsRef, derive_more::Display, Eq, Hash, PartialEq)]
#[as_ref(str)]
pub struct PackageName(String);

impl PackageName {
    pub fn try_new(value: impl Into<String>) -> Result<Self, RunnerIoError> {
        let value = value.into();
        if value.is_empty() {
            return Err(RunnerIoError::InvalidRunnerRequest(
                "package name must not be empty".to_string(),
            ));
        }
        if let Some(invalid) = value
            .chars()
            .find(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')))
        {
            return Err(RunnerIoError::InvalidRunnerRequest(format!(
                "package name contains invalid character '{invalid}'"
            )));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn rust_module_prefix(&self) -> RustModulePrefix {
        RustModulePrefix(self.0.replace('-', "_"))
    }
}

impl PartialEq<&str> for PackageName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<PackageName> for &str {
    fn eq(&self, other: &PackageName) -> bool {
        *self == other.as_str()
    }
}

impl serde::Serialize for PackageName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for PackageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, derive_more::AsRef, derive_more::Display, Eq, Hash, PartialEq)]
#[as_ref(str)]
pub struct RustModulePrefix(String);

impl RustModulePrefix {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct I18nTomlPath(PathBuf);

impl I18nTomlPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, RunnerIoError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(RunnerIoError::InvalidRunnerRequest(
                "i18n.toml path must not be empty".to_string(),
            ));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for I18nTomlPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl serde::Serialize for I18nTomlPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string_lossy())
    }
}

impl<'de> serde::Deserialize<'de> for I18nTomlPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum RunnerRequest {
    Generate {
        crate_name: PackageName,
        i18n_toml_path: I18nTomlPath,
        mode: FluentParseMode,
        dry_run: bool,
    },
    Clean {
        crate_name: PackageName,
        i18n_toml_path: I18nTomlPath,
        all_locales: bool,
        dry_run: bool,
    },
    Check {
        crate_name: PackageName,
        manifest_dir: PathBuf,
    },
}

impl RunnerRequest {
    pub fn crate_name(&self) -> &PackageName {
        match self {
            Self::Generate { crate_name, .. }
            | Self::Clean { crate_name, .. }
            | Self::Check { crate_name, .. } => crate_name,
        }
    }

    pub fn encode(&self) -> Result<String, RunnerIoError> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn decode(encoded: &str) -> Result<Self, RunnerIoError> {
        Ok(serde_json::from_str(encoded)?)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunnerMetadataStore {
    base_dir: PathBuf,
}

impl RunnerMetadataStore {
    pub fn new<T: AsRef<Path>>(base_dir: T) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn temp_for_workspace<T: AsRef<Path>>(workspace_root: T) -> Self {
        Self::new(workspace_root.as_ref().join(".es-fluent"))
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn metadata_dir_path(&self, package_name: &PackageName) -> PathBuf {
        self.base_dir.join("metadata").join(package_name.as_str())
    }

    pub fn ensure_metadata_dir(
        &self,
        package_name: &PackageName,
    ) -> Result<PathBuf, RunnerIoError> {
        let metadata_dir = self.metadata_dir_path(package_name);
        fs::create_dir_all(&metadata_dir)?;
        Ok(metadata_dir)
    }

    pub fn result_path(&self, package_name: &PackageName) -> PathBuf {
        self.metadata_dir_path(package_name).join("result.json")
    }

    pub fn inventory_path(&self, package_name: &PackageName) -> PathBuf {
        self.metadata_dir_path(package_name).join("inventory.json")
    }

    pub fn write_result(
        &self,
        package_name: &PackageName,
        result: &RunnerResult,
    ) -> Result<(), RunnerIoError> {
        self.ensure_metadata_dir(package_name)?;
        let json = serde_json::to_string(result)?;
        fs::write(self.result_path(package_name), json)?;
        Ok(())
    }

    pub fn read_result(&self, package_name: &PackageName) -> Result<RunnerResult, RunnerIoError> {
        let content = fs::read_to_string(self.result_path(package_name))?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn result_changed(&self, package_name: &PackageName) -> bool {
        self.read_result(package_name)
            .map(|result| result.changed)
            .unwrap_or(false)
    }

    pub fn write_inventory(
        &self,
        package_name: &PackageName,
        inventory: &InventoryData,
    ) -> Result<(), RunnerIoError> {
        self.ensure_metadata_dir(package_name)?;
        let json = serde_json::to_string(inventory)?;
        fs::write(self.inventory_path(package_name), json)?;
        Ok(())
    }

    pub fn read_inventory(
        &self,
        package_name: &PackageName,
    ) -> Result<InventoryData, RunnerIoError> {
        let content = fs::read_to_string(self.inventory_path(package_name))?;
        Ok(serde_json::from_str(&content)?)
    }
}

/// Returns a sorted list of locale directory names from an assets directory.
pub fn get_all_locales(assets_dir: &Path) -> Result<Vec<String>, RunnerIoError> {
    let mut locales = Vec::new();

    if !assets_dir.exists() {
        return Ok(locales);
    }

    for entry in fs::read_dir(assets_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            locales.push(name.to_string());
        }
    }

    locales.sort();
    Ok(locales)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str) -> PackageName {
        PackageName::try_new(name).expect("valid package name")
    }

    #[test]
    fn metadata_store_builds_expected_locations() {
        let store = RunnerMetadataStore::new("/tmp/example");
        let package = package("crate-x");
        assert_eq!(
            store.metadata_dir_path(&package),
            Path::new("/tmp/example/metadata/crate-x")
        );
        assert_eq!(
            store.result_path(&package),
            Path::new("/tmp/example/metadata/crate-x/result.json")
        );
        assert_eq!(
            store.inventory_path(&package),
            Path::new("/tmp/example/metadata/crate-x/inventory.json")
        );
        assert_eq!(
            RunnerMetadataStore::temp_for_workspace("/tmp/example").base_dir(),
            Path::new("/tmp/example/.es-fluent")
        );
    }

    #[test]
    fn runner_request_round_trips_through_json() {
        let request = RunnerRequest::Generate {
            crate_name: PackageName::try_new("app").expect("package"),
            i18n_toml_path: I18nTomlPath::new("/tmp/app/i18n.toml").expect("path"),
            mode: FluentParseMode::Aggressive,
            dry_run: true,
        };

        let encoded = request.encode().expect("encode request");
        let decoded = RunnerRequest::decode(&encoded).expect("decode request");

        assert_eq!(decoded, request);
        assert_eq!(decoded.crate_name().as_str(), "app");
    }

    #[test]
    fn runner_request_rejects_invalid_typed_fields() {
        let empty_name =
            RunnerRequest::decode(r#"{"command":"check","crate_name":""}"#).unwrap_err();
        assert!(
            empty_name
                .to_string()
                .contains("package name must not be empty")
        );

        let empty_path = RunnerRequest::decode(
            r#"{"command":"generate","crate_name":"app","i18n_toml_path":"","mode":"conservative","dry_run":false}"#,
        )
        .unwrap_err();
        assert!(
            empty_path
                .to_string()
                .contains("i18n.toml path must not be empty")
        );
    }

    #[test]
    fn write_and_read_result_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let result = RunnerResult { changed: true };
        let store = RunnerMetadataStore::new(temp.path());
        let package = package("crate-x");

        store.write_result(&package, &result).expect("write result");
        let decoded = store.read_result(&package).expect("read result");

        assert_eq!(decoded, result);
        assert!(store.result_changed(&package));
    }

    #[test]
    fn write_and_read_inventory_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = RunnerMetadataStore::new(temp.path());
        let package = package("crate-x");
        let inventory = InventoryData {
            expected_keys: vec![ExpectedKey {
                key: FluentEntryId::try_new("hello").expect("key"),
                variables: vec![FluentArgumentName::try_new("name").expect("variable")],
                resource: Some(ModuleResourceSpec::base("crate-x", true)),
                source_file: SourceFile::new("src/lib.rs"),
                source_line: Some(SourceLine::new(7)),
            }],
        };

        store
            .write_inventory(&package, &inventory)
            .expect("write inventory");
        let decoded = store.read_inventory(&package).expect("read inventory");

        assert_eq!(decoded, inventory);
    }

    #[test]
    fn read_inventory_rejects_invalid_typed_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = RunnerMetadataStore::new(temp.path());
        let package = package("crate-x");
        store.ensure_metadata_dir(&package).expect("metadata dir");
        std::fs::write(
            store.inventory_path(&package),
            r#"{"expected_keys":[{"key":"_invalid","variables":["name"],"source_file":"src/lib.rs","source_line":7}]}"#,
        )
        .expect("write inventory");

        let error = store
            .read_inventory(&package)
            .expect_err("invalid inventory should fail");

        assert!(
            error
                .to_string()
                .contains("Fluent entry id must start with an ASCII letter")
        );
    }

    #[test]
    fn get_all_locales_returns_sorted_directories_only() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("fr")).expect("create fr");
        std::fs::create_dir_all(temp.path().join("en-US")).expect("create en-US");
        std::fs::write(temp.path().join("README.txt"), "ignore me").expect("write file");

        let locales = get_all_locales(temp.path()).expect("get locales");
        assert_eq!(locales, vec!["en-US".to_string(), "fr".to_string()]);
    }
}
