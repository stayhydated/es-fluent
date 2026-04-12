#![doc = include_str!("../README.md")]

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

/// Returns a sorted list of locale directory names from an assets directory.
pub fn get_all_locales(assets_dir: &Path) -> Result<Vec<String>, RunnerIoError> {
    let mut locales = Vec::new();

    if !assets_dir.exists() {
        return Ok(locales);
    }

    for entry in std::fs::read_dir(assets_dir)? {
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

pub fn metadata_dir_path<T: AsRef<Path>>(base_dir: T, crate_name: &str) -> PathBuf {
    base_dir.as_ref().join("metadata").join(crate_name)
}

pub fn ensure_metadata_dir<T: AsRef<Path>>(
    base_dir: T,
    crate_name: &str,
) -> Result<PathBuf, RunnerIoError> {
    let metadata_dir = metadata_dir_path(base_dir, crate_name);
    std::fs::create_dir_all(&metadata_dir)?;
    Ok(metadata_dir)
}

pub fn result_path<T: AsRef<Path>>(base_dir: T, crate_name: &str) -> PathBuf {
    metadata_dir_path(base_dir, crate_name).join("result.json")
}

pub fn inventory_path<T: AsRef<Path>>(base_dir: T, crate_name: &str) -> PathBuf {
    metadata_dir_path(base_dir, crate_name).join("inventory.json")
}

pub fn get_es_fluent_temp_dir<T: AsRef<Path>>(workspace_root: T) -> PathBuf {
    workspace_root.as_ref().join(".es-fluent")
}

pub fn write_result<T: AsRef<Path>>(
    base_dir: T,
    crate_name: &str,
    result: &RunnerResult,
) -> Result<(), RunnerIoError> {
    ensure_metadata_dir(&base_dir, crate_name)?;
    let json = serde_json::to_string(result)?;
    std::fs::write(result_path(base_dir, crate_name), json)?;
    Ok(())
}

pub fn read_result<T: AsRef<Path>>(
    base_dir: T,
    crate_name: &str,
) -> Result<RunnerResult, RunnerIoError> {
    let content = std::fs::read_to_string(result_path(base_dir, crate_name))?;
    Ok(serde_json::from_str(&content)?)
}

pub fn write_inventory<T: AsRef<Path>>(
    base_dir: T,
    crate_name: &str,
    inventory: &InventoryData,
) -> Result<(), RunnerIoError> {
    ensure_metadata_dir(&base_dir, crate_name)?;
    let json = serde_json::to_string(inventory)?;
    std::fs::write(inventory_path(base_dir, crate_name), json)?;
    Ok(())
}

pub fn read_inventory<T: AsRef<Path>>(
    base_dir: T,
    crate_name: &str,
) -> Result<InventoryData, RunnerIoError> {
    let content = std::fs::read_to_string(inventory_path(base_dir, crate_name))?;
    Ok(serde_json::from_str(&content)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_helpers_build_expected_locations() {
        let base = Path::new("/tmp/example");
        assert_eq!(
            metadata_dir_path(base, "crate-x"),
            Path::new("/tmp/example/metadata/crate-x")
        );
        assert_eq!(
            result_path(base, "crate-x"),
            Path::new("/tmp/example/metadata/crate-x/result.json")
        );
        assert_eq!(
            inventory_path(base, "crate-x"),
            Path::new("/tmp/example/metadata/crate-x/inventory.json")
        );
        assert_eq!(
            get_es_fluent_temp_dir(base),
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

        write_result(temp.path(), "crate-x", &result).expect("write result");
        let decoded = read_result(temp.path(), "crate-x").expect("read result");

        assert_eq!(decoded, result);
    }

    #[test]
    fn write_and_read_inventory_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let inventory = InventoryData {
            expected_keys: vec![ExpectedKey {
                key: "hello".to_string(),
                variables: vec!["name".to_string()],
                source_file: Some("src/lib.rs".to_string()),
                source_line: Some(7),
            }],
        };

        write_inventory(temp.path(), "crate-x", &inventory).expect("write inventory");
        let decoded = read_inventory(temp.path(), "crate-x").expect("read inventory");

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
