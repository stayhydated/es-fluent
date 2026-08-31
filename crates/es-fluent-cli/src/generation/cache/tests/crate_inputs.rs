use super::*;

#[test]
fn test_compute_crate_inputs_hash_changes_when_crate_manifest_changes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, None);
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    let second = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, None);

    assert_ne!(first, second);
}

#[test]
fn test_compute_crate_inputs_hash_changes_when_build_script_changes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

    let build_script = temp_dir.path().join("build.rs");
    let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_script));
    fs::write(&build_script, "fn main() {}\n").unwrap();
    let second = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_script));

    assert_ne!(first, second);
}

#[test]
fn test_compute_crate_inputs_hash_tracks_custom_build_modules_but_not_unused_build_rs() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let support_dir = temp_dir.path().join("support");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&support_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
    let build_target = support_dir.join("i18n.rs");
    let helper = support_dir.join("helper.rs");
    fs::write(&build_target, "mod helper; fn main() { helper::run(); }\n").unwrap();
    fs::write(&helper, "pub fn run() {}\n").unwrap();
    fs::write(temp_dir.path().join("build.rs"), "fn main() {}\n").unwrap();

    let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target));
    fs::write(&helper, "pub fn run() { let _changed = true; }\n").unwrap();
    let second = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target));
    assert_ne!(first, second);

    fs::write(
        temp_dir.path().join("build.rs"),
        "fn main() { println!(\"unused\"); }\n",
    )
    .unwrap();
    let third = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target));
    assert_eq!(second, third);
}

#[test]
fn test_compute_crate_inputs_hash_tracks_custom_build_target_outside_package_root() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manifest_dir = temp_dir.path().join("app");
    let src_dir = manifest_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
    let build_target = temp_dir.path().join("shared-build.rs");
    let helper = temp_dir.path().join("shared_helper.rs");
    fs::write(
        &build_target,
        "mod shared_helper; fn main() { shared_helper::run(); }\n",
    )
    .unwrap();
    fs::write(&helper, "pub fn run() {}\n").unwrap();

    let first = compute_crate_inputs_hash(&manifest_dir, &src_dir, None, Some(&build_target))
        .expect("external custom-build graph should be cacheable");
    fs::write(&helper, "pub fn run() { let _changed = true; }\n").unwrap();
    let second = compute_crate_inputs_hash(&manifest_dir, &src_dir, None, Some(&build_target))
        .expect("updated external custom-build graph should be cacheable");

    assert_ne!(first, second);
}

#[test]
fn test_compute_crate_inputs_hash_accepts_explicit_path_submodule_layout() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let support_dir = temp_dir.path().join("support");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&support_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
    let build_target = temp_dir.path().join("build.rs");
    fs::write(
        &build_target,
        "#[path = \"support/helper_impl.rs\"] mod assets; fn main() { assets::run(); }\n",
    )
    .unwrap();
    fs::write(
        support_dir.join("helper_impl.rs"),
        "mod nested; pub fn run() { nested::configure(); }\n",
    )
    .unwrap();
    let nested = support_dir.join("nested.rs");
    fs::write(&nested, "pub fn configure() {}\n").unwrap();

    let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
        .expect("explicit-path submodule graph should be cacheable");
    fs::write(&nested, "pub fn configure() { let _changed = true; }\n").unwrap();
    let second = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
        .expect("updated explicit-path submodule graph should be cacheable");

    assert_ne!(first, second);
}

#[test]
fn test_compute_crate_inputs_hash_accepts_included_submodule_layout() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let support_dir = temp_dir.path().join("support");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&support_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
    let build_target = temp_dir.path().join("build.rs");
    fs::write(
        &build_target,
        "include!(\"support/config.rs\"); fn main() { configure(); }\n",
    )
    .unwrap();
    fs::write(
        support_dir.join("config.rs"),
        "mod nested; fn configure() { nested::run(); }\n",
    )
    .unwrap();
    let nested = support_dir.join("nested.rs");
    fs::write(&nested, "pub fn run() {}\n").unwrap();

    let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
        .expect("included submodule graph should be cacheable");
    fs::write(&nested, "pub fn run() { let _changed = true; }\n").unwrap();
    let second = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
        .expect("updated included submodule graph should be cacheable");

    assert_ne!(first, second);
}

#[test]
fn test_compute_crate_inputs_hash_is_uncacheable_for_indeterminate_build_graph() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
    let build_target = temp_dir.path().join("build.rs");
    fs::write(
        &build_target,
        "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/support.rs\"));\nfn main() {}\n",
    )
    .unwrap();
    fs::write(
        temp_dir.path().join("support.rs"),
        "pub fn configure() {}\n",
    )
    .unwrap();

    assert_eq!(
        compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target)),
        None
    );
}

#[test]
fn test_compute_crate_inputs_hash_is_uncacheable_for_macro_wrapped_include() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let support_dir = temp_dir.path().join("support");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&support_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
    let build_target = temp_dir.path().join("build.rs");
    fs::write(
            &build_target,
            "macro_rules! load_config { () => { include!(\"support/config.rs\"); }; } load_config!(); fn main() {}\n",
        )
        .unwrap();
    fs::write(support_dir.join("config.rs"), "pub fn configure() {}\n").unwrap();

    assert_eq!(
        compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target)),
        None
    );
}

#[test]
fn test_compute_crate_inputs_hash_ignores_generated_dirs_for_root_source_dir() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(temp_dir.path().join("lib.rs"), "pub struct Demo;\n").unwrap();

    let first = compute_crate_inputs_hash(temp_dir.path(), temp_dir.path(), None, None);

    fs::create_dir_all(temp_dir.path().join(".es-fluent/src")).unwrap();
    fs::write(
        temp_dir.path().join(".es-fluent/src/main.rs"),
        "fn main() {}\n",
    )
    .unwrap();
    fs::create_dir_all(temp_dir.path().join("target/debug/build/demo/out")).unwrap();
    fs::write(
        temp_dir
            .path()
            .join("target/debug/build/demo/out/generated.rs"),
        "pub fn generated() {}\n",
    )
    .unwrap();

    let second = compute_crate_inputs_hash(temp_dir.path(), temp_dir.path(), None, None);
    assert_eq!(first, second);

    fs::write(temp_dir.path().join("module.rs"), "pub struct Changed;\n").unwrap();
    let third = compute_crate_inputs_hash(temp_dir.path(), temp_dir.path(), None, None);
    assert_ne!(second, third);
}
