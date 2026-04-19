#![doc = include_str!("../README.md")]

use fs_err as fs;
use std::path::{Path, PathBuf};

mod error;

pub use error::RunnerIoError;

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct RunnerResult {
    pub changed: bool,
}

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct ExpectedKey {
    pub key: String,
    pub variables: Vec<String>,
    pub source_file: Option<String>,
    pub source_line: Option<u32>,
}

#[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct InventoryData {
    pub expected_keys: Vec<ExpectedKey>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerParseMode {
    Conservative,
    Aggressive,
}

#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum RunnerRequest {
    Generate {
        crate_name: String,
        i18n_toml_path: String,
        mode: RunnerParseMode,
        dry_run: bool,
    },
    Clean {
        crate_name: String,
        i18n_toml_path: String,
        all_locales: bool,
        dry_run: bool,
    },
    Check {
        crate_name: String,
    },
}

impl RunnerRequest {
    pub fn crate_name(&self) -> &str {
        match self {
            Self::Generate { crate_name, .. }
            | Self::Clean { crate_name, .. }
            | Self::Check { crate_name } => crate_name,
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

    pub fn metadata_dir_path(&self, crate_name: &str) -> PathBuf {
        self.base_dir.join("metadata").join(crate_name)
    }

    pub fn ensure_metadata_dir(&self, crate_name: &str) -> Result<PathBuf, RunnerIoError> {
        let metadata_dir = self.metadata_dir_path(crate_name);
        fs::create_dir_all(&metadata_dir)?;
        Ok(metadata_dir)
    }

    pub fn result_path(&self, crate_name: &str) -> PathBuf {
        self.metadata_dir_path(crate_name).join("result.json")
    }

    pub fn inventory_path(&self, crate_name: &str) -> PathBuf {
        self.metadata_dir_path(crate_name).join("inventory.json")
    }

    pub fn write_result(
        &self,
        crate_name: &str,
        result: &RunnerResult,
    ) -> Result<(), RunnerIoError> {
        self.ensure_metadata_dir(crate_name)?;
        let json = serde_json::to_string(result)?;
        fs::write(self.result_path(crate_name), json)?;
        Ok(())
    }

    pub fn read_result(&self, crate_name: &str) -> Result<RunnerResult, RunnerIoError> {
        let content = fs::read_to_string(self.result_path(crate_name))?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn result_changed(&self, crate_name: &str) -> bool {
        self.read_result(crate_name)
            .map(|result| result.changed)
            .unwrap_or(false)
    }

    pub fn write_inventory(
        &self,
        crate_name: &str,
        inventory: &InventoryData,
    ) -> Result<(), RunnerIoError> {
        self.ensure_metadata_dir(crate_name)?;
        let json = serde_json::to_string(inventory)?;
        fs::write(self.inventory_path(crate_name), json)?;
        Ok(())
    }

    pub fn read_inventory(&self, crate_name: &str) -> Result<InventoryData, RunnerIoError> {
        let content = fs::read_to_string(self.inventory_path(crate_name))?;
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

    #[test]
    fn metadata_store_builds_expected_locations() {
        let store = RunnerMetadataStore::new("/tmp/example");
        assert_eq!(
            store.metadata_dir_path("crate-x"),
            Path::new("/tmp/example/metadata/crate-x")
        );
        assert_eq!(
            store.result_path("crate-x"),
            Path::new("/tmp/example/metadata/crate-x/result.json")
        );
        assert_eq!(
            store.inventory_path("crate-x"),
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
            crate_name: "app".to_string(),
            i18n_toml_path: "/tmp/app/i18n.toml".to_string(),
            mode: RunnerParseMode::Aggressive,
            dry_run: true,
        };

        let encoded = request.encode().expect("encode request");
        let decoded = RunnerRequest::decode(&encoded).expect("decode request");

        assert_eq!(decoded, request);
        assert_eq!(decoded.crate_name(), "app");
    }

    #[test]
    fn write_and_read_result_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let result = RunnerResult { changed: true };
        let store = RunnerMetadataStore::new(temp.path());

        store
            .write_result("crate-x", &result)
            .expect("write result");
        let decoded = store.read_result("crate-x").expect("read result");

        assert_eq!(decoded, result);
        assert!(store.result_changed("crate-x"));
    }

    #[test]
    fn write_and_read_inventory_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = RunnerMetadataStore::new(temp.path());
        let inventory = InventoryData {
            expected_keys: vec![ExpectedKey {
                key: "hello".to_string(),
                variables: vec!["name".to_string()],
                source_file: Some("src/lib.rs".to_string()),
                source_line: Some(7),
            }],
        };

        store
            .write_inventory("crate-x", &inventory)
            .expect("write inventory");
        let decoded = store.read_inventory("crate-x").expect("read inventory");

        assert_eq!(decoded, inventory);
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
