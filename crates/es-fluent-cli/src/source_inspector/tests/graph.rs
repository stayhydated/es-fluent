use super::*;

#[test]
fn reachable_modules_and_literal_paths_are_followed() {
    let outcome = inspect_fixture(
        &[(
            "lib.rs",
            "mod inline { es_fluent_manager_embedded::define_i18n_module!(); }",
        )],
        "lib.rs",
        SourceTarget::Macro("define_i18n_module", None),
    );
    assert!(matches!(outcome, InspectionOutcome::Found(_)));

    let outcome = inspect_fixture(
        &[
            ("lib.rs", "mod inline { mod registration; }"),
            (
                "inline/registration.rs",
                "es_fluent_manager_embedded::define_i18n_module!();",
            ),
        ],
        "lib.rs",
        SourceTarget::Macro("define_i18n_module", None),
    );
    assert!(matches!(outcome, InspectionOutcome::Found(_)));

    let outcome = inspect_fixture(
        &[
            ("lib.rs", "mod registration;"),
            (
                "registration/mod.rs",
                "es_fluent_manager_embedded::define_i18n_module!();",
            ),
        ],
        "lib.rs",
        SourceTarget::Macro("define_i18n_module", None),
    );
    assert!(matches!(outcome, InspectionOutcome::Found(_)));

    let outcome = inspect_fixture(
        &[
            (
                "build.rs",
                "#[path = \"support/assets.rs\"] mod assets; fn main() { assets::run(); }",
            ),
            (
                "support/assets.rs",
                "pub fn run() { es_fluent_build::track_i18n_assets(); }",
            ),
        ],
        "build.rs",
        SourceTarget::Call("track_i18n_assets"),
    );
    assert!(matches!(outcome, InspectionOutcome::Found(_)));
}

#[test]
fn explicit_path_submodules_resolve_beside_the_explicit_source() {
    let temp = tempfile::tempdir().expect("tempdir");
    let support = temp.path().join("support");
    fs::create_dir_all(&support).expect("create support directory");
    fs::write(
        temp.path().join("build.rs"),
        "#[path = \"support/helper_impl.rs\"] mod assets; fn main() { assets::run(); }\n",
    )
    .expect("write build target");
    fs::write(
        support.join("helper_impl.rs"),
        "mod nested; pub fn run() { nested::configure(); }\n",
    )
    .expect("write explicit module");
    let nested = support.join("nested.rs");
    fs::write(
        &nested,
        "pub fn configure() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write nested module");

    let entry = temp.path().join("build.rs");
    let graph = reachable_source_graph(&entry, temp.path());
    assert!(
        graph.indeterminate_reasons.is_empty(),
        "valid explicit-path graph should be determinate: {:?}",
        graph.indeterminate_reasons
    );
    assert!(
        graph
            .paths
            .contains(&nested.canonicalize().expect("canonical nested module"))
    );
    assert!(matches!(
        inspect(&entry, temp.path(), SourceTarget::Call("track_i18n_assets")),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn explicit_path_modules_may_resolve_beside_the_package() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("app");
    let shared = temp.path().join("shared");
    fs::create_dir_all(&package).expect("create package directory");
    fs::create_dir_all(&shared).expect("create shared directory");
    let entry = package.join("build.rs");
    fs::write(
        &entry,
        "#[path = \"../shared/helper.rs\"] mod helper; fn main() { helper::run(); }\n",
    )
    .expect("write build target");
    let helper = shared.join("helper.rs");
    fs::write(
        &helper,
        "mod nested; pub fn run() { nested::configure(); }\n",
    )
    .expect("write shared helper");
    let nested = shared.join("nested.rs");
    fs::write(
        &nested,
        "pub fn configure() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write nested shared helper");

    let graph = reachable_source_graph(&entry, &package);

    assert!(
        graph.indeterminate_reasons.is_empty(),
        "literal external module should be determinate: {:?}",
        graph.indeterminate_reasons
    );
    assert!(
        graph
            .paths
            .contains(&helper.canonicalize().expect("canonical shared helper"))
    );
    assert!(
        graph
            .paths
            .contains(&nested.canonicalize().expect("canonical nested helper"))
    );
    assert!(matches!(
        inspect(&entry, &package, SourceTarget::Call("track_i18n_assets")),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn unresolved_external_explicit_path_records_nearest_watch_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("app");
    let shared = temp.path().join("shared");
    fs::create_dir_all(&package).expect("create package directory");
    fs::create_dir_all(&shared).expect("create shared directory");
    let entry = package.join("build.rs");
    fs::write(
        &entry,
        "#[path = \"../shared/missing.rs\"] mod helper; fn main() {}\n",
    )
    .expect("write build target");

    let graph = reachable_source_graph(&entry, &package);

    assert!(!graph.indeterminate_reasons.is_empty());
    assert_eq!(graph.watch_dirs, vec![shared]);
}

#[cfg(unix)]
#[test]
fn explicit_path_symlinks_preserve_lexical_and_canonical_sources() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let package = temp.path().join("app");
    let shared = temp.path().join("shared");
    fs::create_dir_all(&package).expect("create package directory");
    fs::create_dir_all(&shared).expect("create shared directory");
    let entry = package.join("build.rs");
    fs::write(
        &entry,
        "#[path = \"helper.rs\"] mod helper; fn main() { helper::run(); }\n",
    )
    .expect("write build target");
    let target = shared.join("helper.rs");
    fs::write(
        &target,
        "include!(\"nested.rs\"); pub fn run() { configure(); }\n",
    )
    .expect("write helper target");
    let lexical_include = package.join("nested.rs");
    fs::write(
        &lexical_include,
        "fn configure() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write lexical include");
    let canonical_sibling = shared.join("nested.rs");
    fs::write(&canonical_sibling, "fn configure() {}\n").expect("write canonical target sibling");
    let lexical = package.join("helper.rs");
    symlink(&target, &lexical).expect("link helper");

    let graph = reachable_source_graph(&entry, &package);

    assert!(
        graph.indeterminate_reasons.is_empty(),
        "symlinked explicit module graph should be determinate: {:?}",
        graph.indeterminate_reasons
    );
    assert!(graph.lexical_paths.contains(&lexical));
    assert!(graph.lexical_paths.contains(&lexical_include));
    assert!(
        graph
            .paths
            .contains(&target.canonicalize().expect("canonical helper target"))
    );
    assert!(
        graph.paths.contains(
            &lexical_include
                .canonicalize()
                .expect("canonical lexical include")
        )
    );
    assert!(
        !graph.paths.contains(
            &canonical_sibling
                .canonicalize()
                .expect("canonical target sibling")
        )
    );
    assert!(matches!(
        inspect(&entry, &package, SourceTarget::Call("track_i18n_assets")),
        InspectionOutcome::Found(_)
    ));

    fs::remove_file(&lexical).expect("remove helper link");
    let missing_graph = reachable_source_graph(&entry, &package);
    assert!(missing_graph.lexical_paths.contains(&lexical));
    assert!(!missing_graph.indeterminate_reasons.is_empty());
    assert_eq!(missing_graph.watch_dirs, vec![package]);
}

#[test]
fn included_submodules_resolve_from_the_include_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let support = temp.path().join("support");
    fs::create_dir_all(&support).expect("create support directory");
    fs::write(
        temp.path().join("build.rs"),
        "include!(\"support/config.rs\"); fn main() { configure(); }\n",
    )
    .expect("write build target");
    fs::write(
        support.join("config.rs"),
        "mod nested; fn configure() { nested::run(); }\n",
    )
    .expect("write included source");
    let nested = support.join("nested.rs");
    fs::write(
        &nested,
        "pub fn run() { es_fluent_build::track_i18n_assets(); }\n",
    )
    .expect("write nested module");

    let entry = temp.path().join("build.rs");
    let graph = reachable_source_graph(&entry, temp.path());
    assert!(
        graph.indeterminate_reasons.is_empty(),
        "valid include graph should be determinate: {:?}",
        graph.indeterminate_reasons
    );
    assert!(
        graph
            .paths
            .contains(&nested.canonicalize().expect("canonical nested module"))
    );
    assert!(matches!(
        inspect(&entry, temp.path(), SourceTarget::Call("track_i18n_assets")),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn unreferenced_files_do_not_count() {
    assert_eq!(
        inspect_fixture(
            &[
                ("lib.rs", "pub struct App;"),
                ("unused.rs", "define_i18n_module!();")
            ],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module", None)
        ),
        InspectionOutcome::NotFound
    );
}
