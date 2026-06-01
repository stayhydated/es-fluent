use anyhow::Result;
use es_fluent_runner::{RunnerIoError, RunnerMetadataStore};
use es_fluent_shared::fluent::{FluentArgumentName, FluentEntryId};
use es_fluent_shared::resource::ModuleResourceSpec;
use es_fluent_shared::source::{SourceFile, SourceLine};
use indexmap::IndexMap;
use std::collections::HashSet;

pub(crate) type ExpectedKeys = IndexMap<FluentEntryId, KeyInfo>;

/// Runtime info about an expected key with its variables and source location.
#[derive(Clone)]
pub(crate) struct KeyInfo {
    pub(crate) variables: HashSet<FluentArgumentName>,
    pub(crate) resource: ModuleResourceSpec,
    pub(crate) source_file: Option<SourceFile>,
    pub(crate) source_line: Option<SourceLine>,
}

/// Read inventory data from the generated inventory.json file.
pub(crate) fn read_inventory_file(
    temp_dir: &std::path::Path,
    crate_name: &str,
) -> Result<ExpectedKeys> {
    let store = RunnerMetadataStore::new(temp_dir);
    let inventory_path = store.inventory_path(crate_name);
    let data = store
        .read_inventory(crate_name)
        .map_err(|error| match error {
            RunnerIoError::Io(_) => anyhow::Error::new(error)
                .context(format!("Failed to read {}", inventory_path.display())),
            RunnerIoError::Json(_) => {
                anyhow::Error::new(error).context("Failed to parse inventory JSON")
            },
            RunnerIoError::InvalidRunnerRequest(_) | RunnerIoError::Message(_) => {
                anyhow::Error::new(error)
            },
        })?;

    let mut expected_keys = IndexMap::new();
    for key_info in data.expected_keys {
        let key = key_info.key;
        let variables = key_info.variables.into_iter().collect::<HashSet<_>>();
        let previous = expected_keys.insert(
            key.clone(),
            KeyInfo {
                variables,
                resource: key_info
                    .resource
                    .unwrap_or_else(|| ModuleResourceSpec::base(crate_name, true)),
                source_file: key_info.source_file,
                source_line: key_info.source_line,
            },
        );
        if previous.is_some() {
            anyhow::bail!(
                "duplicate inventory key '{key}' in {}",
                inventory_path.display()
            );
        }
    }

    Ok(expected_keys)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_inventory_file_parses_expected_key_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let inventory_path = RunnerMetadataStore::new(temp.path()).inventory_path("test-crate");
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
        let hello_key = FluentEntryId::try_new("hello").unwrap();
        let hello = inventory.get(&hello_key).unwrap();
        assert!(
            hello
                .variables
                .contains(&FluentArgumentName::try_new("name").unwrap())
        );
        assert!(
            hello
                .variables
                .contains(&FluentArgumentName::try_new("count").unwrap())
        );
        assert_eq!(
            hello.source_file.as_ref().map(SourceFile::as_str),
            Some("src/lib.rs")
        );
        assert_eq!(hello.source_line.map(SourceLine::get), Some(42));

        let goodbye_key = FluentEntryId::try_new("goodbye").unwrap();
        let goodbye = inventory.get(&goodbye_key).unwrap();
        assert!(goodbye.variables.is_empty());
        assert!(goodbye.source_file.is_none());
        assert!(goodbye.source_line.is_none());
    }

    #[test]
    fn read_inventory_file_rejects_invalid_key_metadata_at_protocol_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let inventory_path = RunnerMetadataStore::new(temp.path()).inventory_path("test-crate");
        fs::create_dir_all(inventory_path.parent().unwrap()).unwrap();
        fs::write(
            &inventory_path,
            r#"{
  "expected_keys": [
    {
      "key": "_invalid",
      "variables": ["name"],
      "source_file": "src/lib.rs",
      "source_line": 42
    }
  ]
}"#,
        )
        .unwrap();

        let error = read_inventory_file(temp.path(), "test-crate")
            .err()
            .expect("invalid key should fail");
        assert!(error.to_string().contains("Failed to parse inventory JSON"));
    }

    #[test]
    fn read_inventory_file_rejects_duplicate_typed_keys() {
        let temp = tempfile::tempdir().unwrap();
        let inventory_path = RunnerMetadataStore::new(temp.path()).inventory_path("test-crate");
        fs::create_dir_all(inventory_path.parent().unwrap()).unwrap();
        fs::write(
            &inventory_path,
            r#"{
  "expected_keys": [
    {
      "key": "hello",
      "variables": ["name"],
      "source_file": "src/lib.rs",
      "source_line": 42
    },
    {
      "key": "hello",
      "variables": ["count"],
      "source_file": "src/other.rs",
      "source_line": 7
    }
  ]
}"#,
        )
        .unwrap();

        let error = read_inventory_file(temp.path(), "test-crate")
            .err()
            .expect("duplicate key should fail");
        assert!(
            error
                .to_string()
                .contains("duplicate inventory key 'hello'")
        );
    }

    #[test]
    fn read_inventory_file_returns_error_for_invalid_json() {
        let temp = tempfile::tempdir().unwrap();
        let inventory_path = RunnerMetadataStore::new(temp.path()).inventory_path("test-crate");
        fs::create_dir_all(inventory_path.parent().unwrap()).unwrap();
        fs::write(&inventory_path, "{invalid-json").unwrap();

        let error = read_inventory_file(temp.path(), "test-crate")
            .err()
            .expect("expected invalid json to fail");
        assert!(error.to_string().contains("Failed to parse inventory JSON"));
    }

    #[test]
    fn read_inventory_file_returns_error_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let error = read_inventory_file(temp.path(), "missing-crate")
            .err()
            .expect("missing inventory should fail");
        assert!(error.to_string().contains("Failed to read"));
    }
}
