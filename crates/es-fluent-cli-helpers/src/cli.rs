//! Inventory collection functionality for CLI commands.

use es_fluent_runner::{ExpectedKey, InventoryData, PackageName, RunnerMetadataStore};
use es_fluent_shared::fluent::{FluentArgumentName, FluentEntryId};
use es_fluent_shared::resource::{ModuleResourceSpec, ResourceRoute};
use es_fluent_shared::source::{SourceFile, SourceLine};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::path::Path;

/// Intermediate metadata for a key during collection.
struct KeyMeta {
    variables: BTreeSet<FluentArgumentName>,
    resource: ModuleResourceSpec,
    source_file: Option<SourceFile>,
    source_line: SourceLine,
    source_description: String,
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
pub fn write_inventory_for_crate(crate_name: &str) -> Result<(), es_fluent_runner::RunnerIoError> {
    let manifest_dir = std::env::current_dir()?;
    write_inventory_for_crate_at(crate_name, &manifest_dir)
}

pub fn write_inventory_for_crate_at(
    crate_name: &str,
    manifest_dir: &Path,
) -> Result<(), es_fluent_runner::RunnerIoError> {
    let package_name = PackageName::try_new(crate_name)?;
    let crate_ident = package_name.rust_module_prefix();

    // Collect all registered type infos for this crate
    let type_infos: Vec<_> = es_fluent::registry::get_all_ftl_type_infos()
        .filter(|info| {
            info.module_path() == crate_ident.as_str()
                || info
                    .module_path()
                    .starts_with(&format!("{}::", crate_ident.as_str()))
        })
        .collect();

    // Build a map of expected keys with their metadata
    let mut keys_map: BTreeMap<FluentEntryId, KeyMeta> = BTreeMap::new();
    for info in &type_infos {
        let resource = ResourceRoute::from_namespace(
            info.try_resolved_namespace(manifest_dir)
                .map_err(|details| {
                    es_fluent_runner::RunnerIoError::Message(format!(
                        "invalid namespace for type '{}': {details}",
                        info.type_name()
                    ))
                })?,
        )
        .resource_spec(crate_name, true);
        for variant in info.variants() {
            let key = variant.entry_id();
            let vars: BTreeSet<FluentArgumentName> = variant.argument_names().into_iter().collect();
            let source_description = info.source_description_for(variant);
            let entry = match keys_map.entry(key.clone()) {
                Entry::Vacant(entry) => entry.insert(KeyMeta {
                    variables: BTreeSet::new(),
                    resource: resource.clone(),
                    source_file: info.source_file(),
                    source_line: variant.source_line(),
                    source_description: source_description.clone(),
                }),
                Entry::Occupied(entry) => {
                    return Err(es_fluent_runner::RunnerIoError::Message(format!(
                        "duplicate generated FTL key '{}' from {} and {}",
                        key.as_str(),
                        entry.get().source_description,
                        source_description
                    )));
                },
            };
            entry.variables.extend(vars);
        }
    }

    // Convert to output format
    let expected_keys: Vec<ExpectedKey> = keys_map
        .into_iter()
        .map(|(key, meta)| ExpectedKey {
            key,
            variables: meta.variables.into_iter().collect(),
            resource: Some(meta.resource),
            source_file: meta.source_file,
            source_line: Some(meta.source_line),
        })
        .collect();

    let data = InventoryData { expected_keys };

    RunnerMetadataStore::new(Path::new(".")).write_inventory(&package_name, &data)
}

#[cfg(test)]
#[serial_test::serial(process)]
mod tests {
    use super::*;
    use es_fluent::registry::{
        FtlTypeInfo, FtlVariant, NamespaceRule, RegisteredFtlType, StaticFluentArgumentName,
        StaticFluentEntryId,
    };
    use es_fluent_shared::meta::TypeKind;
    use std::borrow::Cow;

    static VARIANTS: &[FtlVariant] = &[
        FtlVariant::new(
            "Primary",
            StaticFluentEntryId::new_unchecked("my_key"),
            &[
                StaticFluentArgumentName::new_unchecked("name"),
                StaticFluentArgumentName::new_unchecked("count"),
            ],
            "test_crate",
            42,
        ),
        FtlVariant::new(
            "Secondary",
            StaticFluentEntryId::new_unchecked("secondary_key"),
            &[StaticFluentArgumentName::new_unchecked("extra")],
            "test_crate",
            55,
        ),
    ];

    static INFO: FtlTypeInfo = FtlTypeInfo::new(
        TypeKind::Struct,
        "InventoryType",
        VARIANTS,
        "src/lib.rs",
        "test_crate",
        Some(NamespaceRule::Literal(Cow::Borrowed("ui"))),
    );

    es_fluent::__inventory::submit! {
        RegisteredFtlType(&INFO)
    }

    static DUPLICATE_VARIANTS: &[FtlVariant] = &[
        FtlVariant::new(
            "Primary",
            StaticFluentEntryId::new_unchecked("duplicated_key"),
            &[StaticFluentArgumentName::new_unchecked("name")],
            "test_crate_duplicate_inventory",
            42,
        ),
        FtlVariant::new(
            "Secondary",
            StaticFluentEntryId::new_unchecked("duplicated_key"),
            &[StaticFluentArgumentName::new_unchecked("extra")],
            "test_crate_duplicate_inventory",
            55,
        ),
    ];

    static DUPLICATE_INFO: FtlTypeInfo = FtlTypeInfo::new(
        TypeKind::Struct,
        "DuplicateInventoryType",
        DUPLICATE_VARIANTS,
        "src/lib.rs",
        "test_crate_duplicate_inventory",
        Some(NamespaceRule::Literal(Cow::Borrowed("ui"))),
    );

    es_fluent::__inventory::submit! {
        RegisteredFtlType(&DUPLICATE_INFO)
    }

    static VARIANTS_NO_FILE: &[FtlVariant] = &[FtlVariant::new(
        "NoFilePath",
        StaticFluentEntryId::new_unchecked("empty_file_key"),
        &[],
        "test_crate_empty_file",
        7,
    )];

    static INFO_NO_FILE: FtlTypeInfo = FtlTypeInfo::new(
        TypeKind::Struct,
        "InventoryTypeNoFile",
        VARIANTS_NO_FILE,
        "",
        "test_crate_empty_file",
        None,
    );

    es_fluent::__inventory::submit! {
        RegisteredFtlType(&INFO_NO_FILE)
    }

    fn with_temp_cwd<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
        let original = std::env::current_dir().expect("cwd");
        let temp = tempfile::tempdir().expect("tempdir");
        std::env::set_current_dir(temp.path()).expect("set cwd");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(temp.path())));
        std::env::set_current_dir(original).expect("restore cwd");

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn write_inventory_for_crate_writes_expected_key_data() {
        with_temp_cwd(|cwd| {
            write_inventory_for_crate("test-crate").expect("write inventory");

            let inventory_path = cwd.join("metadata/test-crate/inventory.json");
            let content = std::fs::read_to_string(inventory_path).expect("read inventory");
            let json: serde_json::Value = serde_json::from_str(&content).expect("parse json");

            let keys = json["expected_keys"]
                .as_array()
                .expect("expected_keys array");
            assert_eq!(keys.len(), 2);

            let key = &keys[0];
            assert_eq!(key["key"], "my_key");
            assert_eq!(key["resource"]["key"], "test-crate/ui");
            assert_eq!(key["resource"]["locale_relative_path"], "test-crate/ui.ftl");
            assert_eq!(key["source_file"], "src/lib.rs");
            assert_eq!(key["source_line"], 42);

            let vars: Vec<_> = key["variables"]
                .as_array()
                .expect("variables array")
                .iter()
                .filter_map(|value| value.as_str())
                .collect();
            assert_eq!(vars, vec!["count", "name"]);

            let key = &keys[1];
            assert_eq!(key["key"], "secondary_key");
            assert_eq!(key["resource"]["key"], "test-crate/ui");
            assert_eq!(key["resource"]["locale_relative_path"], "test-crate/ui.ftl");
            assert_eq!(key["source_file"], "src/lib.rs");
            assert_eq!(key["source_line"], 55);
            let vars: Vec<_> = key["variables"]
                .as_array()
                .expect("variables array")
                .iter()
                .filter_map(|value| value.as_str())
                .collect();
            assert_eq!(vars, vec!["extra"]);
        });
    }

    #[test]
    fn write_inventory_rejects_duplicate_registered_keys() {
        with_temp_cwd(|_| {
            let err = write_inventory_for_crate("test-crate-duplicate-inventory")
                .expect_err("duplicate inventory should fail");

            let message = err.to_string();
            assert!(message.contains("duplicate generated FTL key 'duplicated_key'"));
            assert!(message.contains("DuplicateInventoryType"));
            assert!(message.contains("Primary"));
            assert!(message.contains("Secondary"));
            assert!(message.contains("src/lib.rs:42"));
            assert!(message.contains("src/lib.rs:55"));
        });
    }

    #[test]
    fn write_inventory_for_unknown_crate_writes_empty_result() {
        with_temp_cwd(|cwd| {
            write_inventory_for_crate("unknown-crate").expect("write inventory");
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
            write_inventory_for_crate("test-crate-empty-file").expect("write inventory");

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

    #[test]
    fn static_inventory_wrappers_validate_manual_values() {
        assert!(
            StaticFluentEntryId::try_new("_invalid")
                .expect_err("invalid message id")
                .to_string()
                .contains("must start with an ASCII letter")
        );
        assert!(
            StaticFluentArgumentName::try_new("not valid")
                .expect_err("invalid arg")
                .to_string()
                .contains("contains invalid character")
        );
    }
}
