use anyhow::{Context as _, Result};
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;

/// Expected key information from inventory (deserialized from temp crate output).
#[derive(Deserialize)]
struct ExpectedKey {
    key: String,
    variables: Vec<String>,
    /// The Rust source file where this key is defined.
    source_file: Option<String>,
    /// The line number in the Rust source file.
    source_line: Option<u32>,
}

/// Runtime info about an expected key with its variables and source location.
#[derive(Clone)]
pub(crate) struct KeyInfo {
    pub(crate) variables: HashSet<String>,
    pub(crate) source_file: Option<String>,
    pub(crate) source_line: Option<u32>,
}

/// The inventory data output from the temp crate.
#[derive(Deserialize)]
struct InventoryData {
    expected_keys: Vec<ExpectedKey>,
}

/// Read inventory data from the generated inventory.json file.
pub(crate) fn read_inventory_file(
    temp_dir: &std::path::Path,
    crate_name: &str,
) -> Result<IndexMap<String, KeyInfo>> {
    let inventory_path = es_fluent_derive_core::get_metadata_inventory_path(temp_dir, crate_name);
    let json_str = fs::read_to_string(&inventory_path)
        .with_context(|| format!("Failed to read {}", inventory_path.display()))?;

    let data: InventoryData =
        serde_json::from_str(&json_str).context("Failed to parse inventory JSON")?;

    // Convert to IndexMap with KeyInfo for richer metadata
    let mut expected_keys = IndexMap::new();
    for key_info in data.expected_keys {
        expected_keys.insert(
            key_info.key,
            KeyInfo {
                variables: key_info.variables.into_iter().collect(),
                source_file: key_info.source_file,
                source_line: key_info.source_line,
            },
        );
    }

    Ok(expected_keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn read_inventory_file_parses_expected_key_metadata() {
        let temp = tempdir().unwrap();
        let inventory_path =
            es_fluent_derive_core::get_metadata_inventory_path(temp.path(), "test-crate");
        fs::create_dir_all(inventory_path.parent().unwrap()).unwrap();
        fs::write(
            &inventory_path,
            r#"{
  "expected_keys": [
    {
      "key": "hello",
      "variables": ["name", "count"],
      "source_file": "src/lib.rs",
      "source_line": 42
    },
    {
      "key": "goodbye",
      "variables": [],
      "source_file": null,
      "source_line": null
    }
  ]
}"#,
        )
        .unwrap();

        let inventory = read_inventory_file(temp.path(), "test-crate").unwrap();

        assert_eq!(inventory.len(), 2);
        let hello = inventory.get("hello").unwrap();
        assert!(hello.variables.contains("name"));
        assert!(hello.variables.contains("count"));
        assert_eq!(hello.source_file.as_deref(), Some("src/lib.rs"));
        assert_eq!(hello.source_line, Some(42));

        let goodbye = inventory.get("goodbye").unwrap();
        assert!(goodbye.variables.is_empty());
        assert!(goodbye.source_file.is_none());
        assert!(goodbye.source_line.is_none());
    }

    #[test]
    fn read_inventory_file_returns_error_for_invalid_json() {
        let temp = tempdir().unwrap();
        let inventory_path =
            es_fluent_derive_core::get_metadata_inventory_path(temp.path(), "test-crate");
        fs::create_dir_all(inventory_path.parent().unwrap()).unwrap();
        fs::write(&inventory_path, "{invalid-json").unwrap();

        let error = read_inventory_file(temp.path(), "test-crate")
            .err()
            .expect("expected invalid json to fail");
        assert!(error.to_string().contains("Failed to parse inventory JSON"));
    }

    #[test]
    fn read_inventory_file_returns_error_when_missing() {
        let temp = tempdir().unwrap();
        let error = read_inventory_file(temp.path(), "missing-crate")
            .err()
            .expect("missing inventory should fail");
        assert!(error.to_string().contains("Failed to read"));
    }
}
