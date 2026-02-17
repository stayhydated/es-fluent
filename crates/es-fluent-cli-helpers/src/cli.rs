//! Inventory collection functionality for CLI commands.

use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Expected key information from inventory.
#[derive(Serialize)]
pub struct ExpectedKey {
    pub key: String,
    pub variables: Vec<String>,
    /// The Rust source file where this key is defined.
    pub source_file: Option<String>,
    /// The line number in the Rust source file.
    pub source_line: Option<u32>,
}

/// The inventory data output.
#[derive(Serialize)]
pub struct InventoryData {
    pub expected_keys: Vec<ExpectedKey>,
}

/// Intermediate metadata for a key during collection.
#[derive(Default)]
struct KeyMeta {
    variables: HashSet<String>,
    source_file: Option<String>,
    source_line: Option<u32>,
}

/// Collects inventory data for a crate and writes it to `inventory.json`.
///
/// This function is used by the es-fluent CLI to collect expected FTL keys
/// and their variables from inventory-registered types.
///
/// # Arguments
///
/// * `crate_name` - The name of the crate to collect inventory for (e.g., "my-crate")
///
/// # Panics
///
/// Panics if serialization or file writing fails.
pub fn write_inventory_for_crate(crate_name: &str) {
    let crate_ident = crate_name.replace('-', "_");

    // Collect all registered type infos for this crate
    let type_infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| {
            info.module_path == crate_ident
                || info.module_path.starts_with(&format!("{}::", crate_ident))
        })
        .collect();

    // Build a map of expected keys with their metadata
    let mut keys_map: HashMap<String, KeyMeta> = HashMap::new();
    for info in &type_infos {
        for variant in info.variants {
            let key = variant.ftl_key.to_string();
            let vars: HashSet<String> = variant.args.iter().map(|s| s.to_string()).collect();
            let entry = keys_map.entry(key).or_insert_with(|| KeyMeta {
                variables: HashSet::new(),
                source_file: if info.file_path.is_empty() {
                    None
                } else {
                    Some(info.file_path.to_string())
                },
                source_line: Some(variant.line),
            });
            entry.variables.extend(vars);
            // Keep the first source location we encounter
        }
    }

    // Convert to output format
    let expected_keys: Vec<ExpectedKey> = keys_map
        .into_iter()
        .map(|(key, meta)| ExpectedKey {
            key,
            variables: meta.variables.into_iter().collect(),
            source_file: meta.source_file,
            source_line: meta.source_line,
        })
        .collect();

    let data = InventoryData { expected_keys };

    // Write inventory data to file
    let json = serde_json::to_string(&data).expect("Failed to serialize inventory data");

    let metadata_dir = es_fluent_derive_core::create_metadata_dir(crate_name)
        .expect("Failed to create metadata directory");
    let inventory_path = metadata_dir.join("inventory.json");

    std::fs::write(&inventory_path, json).expect("Failed to write inventory file");
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent::registry::{FtlTypeInfo, FtlVariant, NamespaceRule, RegisteredFtlType};
    use es_fluent_derive_core::meta::TypeKind;
    use tempfile::tempdir;

    static VARIANTS: &[FtlVariant] = &[
        FtlVariant {
            name: "Primary",
            ftl_key: "my_key",
            args: &["name", "count"],
            module_path: "test_crate",
            line: 42,
        },
        FtlVariant {
            name: "Secondary",
            ftl_key: "my_key",
            args: &["extra"],
            module_path: "test_crate",
            line: 55,
        },
    ];

    static INFO: FtlTypeInfo = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "InventoryType",
        variants: VARIANTS,
        file_path: "src/lib.rs",
        module_path: "test_crate",
        namespace: Some(NamespaceRule::Literal("ui")),
    };

    es_fluent::__inventory::submit! {
        RegisteredFtlType(&INFO)
    }

    static VARIANTS_NO_FILE: &[FtlVariant] = &[FtlVariant {
        name: "NoFilePath",
        ftl_key: "empty_file_key",
        args: &[],
        module_path: "test_crate_empty_file",
        line: 7,
    }];

    static INFO_NO_FILE: FtlTypeInfo = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "InventoryTypeNoFile",
        variants: VARIANTS_NO_FILE,
        file_path: "",
        module_path: "test_crate_empty_file",
        namespace: None,
    };

    es_fluent::__inventory::submit! {
        RegisteredFtlType(&INFO_NO_FILE)
    }

    fn with_temp_cwd<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
        let _guard = crate::TEST_CWD_LOCK.lock().expect("lock poisoned");
        let original = std::env::current_dir().expect("cwd");
        let temp = tempdir().expect("tempdir");
        std::env::set_current_dir(temp.path()).expect("set cwd");
        let result = f(temp.path());
        std::env::set_current_dir(original).expect("restore cwd");
        result
    }

    #[test]
    fn write_inventory_for_crate_writes_expected_key_data() {
        with_temp_cwd(|cwd| {
            write_inventory_for_crate("test-crate");

            let inventory_path = cwd.join("metadata/test-crate/inventory.json");
            let content = std::fs::read_to_string(inventory_path).expect("read inventory");
            let json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

            let keys = json["expected_keys"]
                .as_array()
                .expect("expected_keys array");
            assert_eq!(keys.len(), 1);

            let key = &keys[0];
            assert_eq!(key["key"], "my_key");
            assert_eq!(key["source_file"], "src/lib.rs");
            assert_eq!(key["source_line"], 42);

            let mut vars: Vec<_> = key["variables"]
                .as_array()
                .expect("variables array")
                .iter()
                .filter_map(|value| value.as_str())
                .collect();
            vars.sort_unstable();
            assert_eq!(vars, vec!["count", "extra", "name"]);
        });
    }

    #[test]
    fn write_inventory_for_unknown_crate_writes_empty_result() {
        with_temp_cwd(|cwd| {
            write_inventory_for_crate("unknown-crate");
            let inventory_path = cwd.join("metadata/unknown-crate/inventory.json");
            let content = std::fs::read_to_string(inventory_path).expect("read inventory");
            let json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

            assert_eq!(
                json["expected_keys"]
                    .as_array()
                    .expect("expected_keys")
                    .len(),
                0
            );
        });
    }

    #[test]
    fn write_inventory_sets_source_file_to_null_when_missing() {
        with_temp_cwd(|cwd| {
            write_inventory_for_crate("test-crate-empty-file");

            let inventory_path = cwd.join("metadata/test-crate-empty-file/inventory.json");
            let content = std::fs::read_to_string(inventory_path).expect("read inventory");
            let json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

            let keys = json["expected_keys"]
                .as_array()
                .expect("expected_keys array");
            assert_eq!(keys.len(), 1);

            let key = &keys[0];
            assert_eq!(key["key"], "empty_file_key");
            assert!(key["source_file"].is_null());
            assert_eq!(key["source_line"], 7);
        });
    }
}
