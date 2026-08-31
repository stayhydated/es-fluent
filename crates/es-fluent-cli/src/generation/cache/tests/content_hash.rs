use super::*;

#[test]
fn test_compute_content_hash_without_i18n_toml() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let hash1 = compute_content_hash(&src_dir, None);
    let hash2 = compute_content_hash(&src_dir, None);

    // Same content should produce same hash
    assert_eq!(hash1, hash2);
    assert!(!hash1.is_empty());
}

#[test]
fn test_compute_content_hash_with_i18n_toml() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let i18n_path = temp_dir.path().join("i18n.toml");
    fs::write(&i18n_path, I18N_CONFIG).unwrap();

    let hash_with_toml = compute_content_hash(&src_dir, Some(&i18n_path));
    let hash_without_toml = compute_content_hash(&src_dir, None);

    // Hash should differ when i18n.toml is included
    assert_ne!(hash_with_toml, hash_without_toml);
}

#[test]
fn test_compute_content_hash_changes_when_i18n_toml_changes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let i18n_path = temp_dir.path().join("i18n.toml");
    fs::write(&i18n_path, I18N_CONFIG).unwrap();

    let hash1 = compute_content_hash(&src_dir, Some(&i18n_path));

    // Change the i18n.toml content (e.g., changing fluent_feature)
    fs::write(
        &i18n_path,
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\nfluent_feature = [\"i18n\"]",
    )
    .unwrap();

    let hash2 = compute_content_hash(&src_dir, Some(&i18n_path));

    // Hash should change when i18n.toml content changes
    assert_ne!(hash1, hash2);
}

#[test]
fn test_compute_content_hash_unchanged_when_rs_unchanged() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let i18n_path = temp_dir.path().join("i18n.toml");
    fs::write(&i18n_path, I18N_CONFIG).unwrap();

    let hash1 = compute_content_hash(&src_dir, Some(&i18n_path));

    // Re-write same content (simulates save without changes)
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();
    fs::write(&i18n_path, I18N_CONFIG).unwrap();

    let hash2 = compute_content_hash(&src_dir, Some(&i18n_path));

    // Hash should remain the same when content is identical
    assert_eq!(hash1, hash2);
}

#[test]
fn test_compute_content_hash_nonexistent_i18n_toml() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let nonexistent_path = temp_dir.path().join("nonexistent.toml");

    // Should not panic and should produce same hash as None
    let hash_with_nonexistent = compute_content_hash(&src_dir, Some(&nonexistent_path));
    let hash_without = compute_content_hash(&src_dir, None);

    assert_eq!(hash_with_nonexistent, hash_without);
}

#[test]
fn test_compute_content_hash_ignores_path_aliases() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let i18n_path = temp_dir.path().join("i18n.toml");
    fs::write(&i18n_path, I18N_CONFIG).unwrap();

    let aliased_src_dir = temp_dir.path().join("src").join(".");
    let aliased_i18n_path = temp_dir.path().join(".").join("i18n.toml");

    let direct_hash = compute_content_hash(&src_dir, Some(&i18n_path));
    let aliased_hash = compute_content_hash(&aliased_src_dir, Some(&aliased_i18n_path));

    assert_eq!(direct_hash, aliased_hash);
}

#[test]
fn test_compute_content_hash_only_rs_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let hash1 = compute_content_hash(&src_dir, None);

    // Add a non-.rs file - should not affect hash
    fs::write(src_dir.join("notes.txt"), "some notes").unwrap();

    let hash2 = compute_content_hash(&src_dir, None);

    assert_eq!(hash1, hash2);
}
