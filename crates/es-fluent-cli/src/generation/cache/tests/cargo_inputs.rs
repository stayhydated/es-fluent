use super::*;

#[test]
fn test_compute_workspace_inputs_hash_changes_when_manifest_changes() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = []\n",
    )
    .unwrap();

    let first = compute_workspace_inputs_hash(temp_dir.path());
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = []\nresolver = \"3\"\n",
    )
    .unwrap();
    let second = compute_workspace_inputs_hash(temp_dir.path());

    assert_ne!(first, second);
}

#[test]
fn test_compute_workspace_inputs_hash_changes_when_lockfile_changes() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = []\n",
    )
    .unwrap();
    fs::write(temp_dir.path().join("Cargo.lock"), "version = 4\n").unwrap();

    let first = compute_workspace_inputs_hash(temp_dir.path());
    fs::write(temp_dir.path().join("Cargo.lock"), "version = 5\n").unwrap();
    let second = compute_workspace_inputs_hash(temp_dir.path());

    assert_ne!(first, second);
}

#[test]
fn test_compute_workspace_inputs_hash_tracks_ancestor_and_cargo_home_configs() {
    let temp_dir = tempfile::tempdir().unwrap();
    let ancestor = temp_dir.path().join("ancestor");
    let workspace_root = ancestor.join("workspace");
    let cargo_home = temp_dir.path().join("cargo-home");
    fs::create_dir_all(&workspace_root).unwrap();
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = []\n",
    )
    .unwrap();

    let initial =
        compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home.clone()));

    let ancestor_cargo = ancestor.join(".cargo");
    fs::create_dir_all(&ancestor_cargo).unwrap();
    fs::write(
        ancestor_cargo.join("config.toml"),
        "[env]\nINVENTORY_MODE = \"off\"\n",
    )
    .unwrap();
    let with_ancestor =
        compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home.clone()));
    assert_ne!(initial, with_ancestor);

    fs::create_dir_all(&cargo_home).unwrap();
    fs::write(
        cargo_home.join("config"),
        "[build]\nrustflags = [\"--cfg\", \"inventory_on\"]\n",
    )
    .unwrap();
    let with_cargo_home =
        compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home));
    assert_ne!(with_ancestor, with_cargo_home);
}

#[test]
fn test_compute_workspace_inputs_hash_tracks_recursive_configs_and_configured_lockfiles() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace_root = temp_dir.path().join("workspace");
    let cargo_home = temp_dir.path().join("cargo-home");
    let cargo_dir = workspace_root.join(".cargo");
    let config_parts = workspace_root.join("config-parts");
    let lock_dir = workspace_root.join("locks");
    fs::create_dir_all(&cargo_dir).unwrap();
    fs::create_dir_all(&config_parts).unwrap();
    fs::create_dir_all(&lock_dir).unwrap();
    fs::write(
        workspace_root.join("Cargo.toml"),
        "[workspace]\nmembers = []\n",
    )
    .unwrap();
    fs::write(
        cargo_dir.join("config.toml"),
        concat!(
            "include = [\n",
            "  \"../config-parts/base.toml\",\n",
            "  { path = \"../optional/config.toml\", optional = true },\n",
            "]\n",
        ),
    )
    .unwrap();
    fs::write(
        config_parts.join("base.toml"),
        concat!(
            "include = [\"nested.toml\"]\n",
            "[resolver]\n",
            "lockfile-path = \"locks/Cargo.lock\"\n",
        ),
    )
    .unwrap();
    let nested_config = config_parts.join("nested.toml");
    fs::write(&nested_config, "[env]\nINVENTORY_MODE = \"off\"\n").unwrap();
    let configured_lockfile = lock_dir.join("Cargo.lock");
    fs::write(&configured_lockfile, "version = 4\n").unwrap();

    let initial =
        compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home.clone()));

    fs::write(&nested_config, "[env]\nINVENTORY_MODE = \"on\"\n").unwrap();
    let with_nested_change =
        compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home.clone()));
    assert_ne!(initial, with_nested_change);

    fs::write(&configured_lockfile, "version = 5\n").unwrap();
    let with_lockfile_change =
        compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home.clone()));
    assert_ne!(with_nested_change, with_lockfile_change);

    let optional_config = workspace_root.join("optional/config.toml");
    fs::create_dir_all(optional_config.parent().unwrap()).unwrap();
    fs::write(
        &optional_config,
        "[build]\nrustflags = [\"--cfg\", \"extra\"]\n",
    )
    .unwrap();
    let with_optional_config =
        compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home));
    assert_ne!(with_lockfile_change, with_optional_config);
}
