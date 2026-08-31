use super::*;
use es_fluent_runner::PackageName;
use indexmap::IndexMap;

#[test]
fn metadata_cache_save_load_and_validity_round_trip() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(temp_dir.path().join("Cargo.lock"), "lock-content").unwrap();

    let cache = MetadataCache {
        cargo_lock_hash: MetadataCache::hash_cargo_lock(temp_dir.path()).unwrap(),
        es_fluent_dep: cargo_manifest::Dependency::Detailed(cargo_manifest::DependencyDetail {
            path: Some("../es-fluent".to_string()),
            ..Default::default()
        }),
        es_fluent_cli_helpers_dep: cargo_manifest::Dependency::Detailed(
            cargo_manifest::DependencyDetail {
                path: Some("../helpers".to_string()),
                ..Default::default()
            },
        ),
    };
    cache.save(temp_dir.path()).unwrap();

    let cache_path = temp_dir.path().join(MetadataCache::CACHE_FILE);
    let mut legacy_cache: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&cache_path).unwrap()).unwrap();
    legacy_cache.as_object_mut().unwrap().insert(
        "target_dir".to_string(),
        serde_json::json!("obsolete-target"),
    );
    fs::write(&cache_path, serde_json::to_vec(&legacy_cache).unwrap()).unwrap();

    let loaded = MetadataCache::load(temp_dir.path()).unwrap();
    assert_eq!(loaded.es_fluent_dep, cache.es_fluent_dep);
    assert!(loaded.is_valid(temp_dir.path()));

    fs::write(temp_dir.path().join("Cargo.lock"), "changed-lock-content").unwrap();
    assert!(!loaded.is_valid(temp_dir.path()));
}

#[test]
fn runner_cache_save_and_load_round_trip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut hashes = IndexMap::new();
    hashes.insert(
        PackageName::try_new("test-crate").expect("valid package name"),
        "abc123".to_string(),
    );

    let cache = RunnerCache {
        crate_hashes: hashes.clone(),
        runner_mtime: 42,
        cli_version: "0.1.0".to_string(),
        runner_protocol_version: es_fluent_runner::RUNNER_PROTOCOL_VERSION,
        workspace_inputs_hash: "workspace-hash".to_string(),
    };
    cache.save(temp_dir.path()).unwrap();

    let loaded = RunnerCache::load(temp_dir.path()).unwrap();
    assert_eq!(loaded.runner_mtime, 42);
    assert_eq!(loaded.cli_version, "0.1.0");
    assert_eq!(
        loaded.runner_protocol_version,
        es_fluent_runner::RUNNER_PROTOCOL_VERSION
    );
    assert_eq!(loaded.crate_hashes, hashes);
    assert_eq!(loaded.workspace_inputs_hash, "workspace-hash");
}
