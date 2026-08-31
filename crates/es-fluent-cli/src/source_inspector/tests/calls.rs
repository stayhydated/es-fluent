use super::*;

#[test]
fn direct_qualified_and_imported_calls_are_found() {
    for source in [
        "fn main() { es_fluent_build::track_i18n_assets(); }",
        "use es_fluent_build::track_i18n_assets; fn main() { track_i18n_assets(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }
}

#[test]
fn build_helper_wrappers_imported_from_modules_are_reachable() {
    for import_and_call in [
        "use helper::setup; fn main() { setup(); }",
        "use helper::setup as configure; fn main() { configure(); }",
        "fn main() { use helper::setup as configure; configure(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[
                    ("build.rs", &format!("mod helper; {import_and_call}")),
                    (
                        "helper.rs",
                        "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                    ),
                ],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }
}

#[test]
fn qualified_calls_through_imported_module_bindings_are_reachable() {
    for import_and_call in [
        "use crate::helper as h; pub fn configure() { h::setup(); }",
        "use crate::helper; pub fn configure() { helper::setup(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[
                    (
                        "build.rs",
                        "mod helper; mod nested; fn main() { nested::configure(); }",
                    ),
                    (
                        "helper.rs",
                        "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                    ),
                    ("nested.rs", import_and_call),
                ],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }
}

#[test]
fn path_namespace_bindings_shadow_qualified_import_prefixes() {
    for (main_call, nested_source) in [
        (
            "nested::configure();",
            concat!(
                "use crate::helper as h; ",
                "pub fn configure() { ",
                "struct h; impl h { fn setup() {} } h::setup(); ",
                "}",
            ),
        ),
        (
            "nested::configure();",
            concat!(
                "use crate::helper as h; ",
                "pub fn configure() { ",
                "h::setup(); struct h; impl h { fn setup() {} } ",
                "}",
            ),
        ),
        (
            "nested::configure();",
            concat!(
                "use crate::helper as h; ",
                "pub fn configure() { ",
                "mod h { pub fn setup() {} } h::setup(); ",
                "}",
            ),
        ),
        (
            "nested::configure();",
            concat!(
                "use crate::helper as h; ",
                "struct Local; impl Local { fn setup() {} } ",
                "pub fn configure() { type h = Local; h::setup(); }",
            ),
        ),
        (
            "nested::configure::<nested::Local>();",
            concat!(
                "use crate::helper as h; ",
                "pub trait Setup { fn setup(); } ",
                "pub struct Local; impl Setup for Local { fn setup() {} } ",
                "pub fn configure<h: Setup>() { h::setup(); }",
            ),
        ),
    ] {
        let outcome = inspect_fixture(
            &[
                (
                    "build.rs",
                    &format!("mod helper; mod nested; fn main() {{ {main_call} }}"),
                ),
                (
                    "helper.rs",
                    "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                ),
                ("nested.rs", nested_source),
            ],
            "build.rs",
            SourceTarget::Call("track_i18n_assets"),
        );

        assert!(matches!(outcome, InspectionOutcome::Indeterminate(_)));
    }
}

#[test]
fn value_namespace_bindings_do_not_shadow_qualified_import_prefixes() {
    for (main_call, nested_source) in [
        (
            "nested::configure();",
            "use crate::helper as h; pub fn configure() { let h = (); h::setup(); }",
        ),
        (
            "nested::configure::<0>();",
            "use crate::helper as h; pub fn configure<const h: usize>() { h::setup(); }",
        ),
    ] {
        assert!(matches!(
            inspect_fixture(
                &[
                    (
                        "build.rs",
                        &format!("mod helper; mod nested; fn main() {{ {main_call} }}"),
                    ),
                    (
                        "helper.rs",
                        "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                    ),
                    ("nested.rs", nested_source),
                ],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }
}

#[test]
fn grouped_self_aliases_and_repeated_super_imports_are_reachable() {
    for (build_source, outer_source) in [
        (
            "mod helper; mod outer; fn main() { outer::configure(); }",
            "use crate::helper::{self as h}; pub fn configure() { h::setup(); }",
        ),
        (
            "mod helper; mod outer; fn main() { outer::nested::configure(); }",
            "pub mod nested { use super::super::helper::setup; pub fn configure() { setup(); } }",
        ),
    ] {
        assert!(matches!(
            inspect_fixture(
                &[
                    ("build.rs", build_source),
                    (
                        "helper.rs",
                        "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
                    ),
                    ("outer.rs", outer_source),
                ],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Found(_)
        ));
    }
}

#[test]
fn leading_absolute_calls_do_not_resolve_to_local_modules() {
    let outcome = inspect_fixture(
        &[
            ("build.rs", "mod helper; fn main() { ::helper::setup(); }"),
            (
                "helper.rs",
                "pub fn setup() { es_fluent_build::track_i18n_assets(); }",
            ),
        ],
        "build.rs",
        SourceTarget::Call("track_i18n_assets"),
    );

    assert!(matches!(outcome, InspectionOutcome::Indeterminate(_)));
}

#[test]
fn qualified_and_imported_macros_are_found() {
    for source in [
        "es_fluent_manager_embedded::define_i18n_module!();",
        "use es_fluent_manager_embedded::define_i18n_module; define_i18n_module!();",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("lib.rs", source)],
                "lib.rs",
                SourceTarget::Macro("define_i18n_module", None)
            ),
            InspectionOutcome::Found(_)
        ));
    }
}

#[test]
fn manager_macros_must_match_declared_dependency_roots() {
    for source in [
        "es_fluent_manager_bevy::define_i18n_module!();",
        "use es_fluent_manager_bevy::define_i18n_module; define_i18n_module!();",
        "es_fluent_manager_embedded::define_i18n_module!(); es_fluent_manager_bevy::define_i18n_module!();",
    ] {
        assert_eq!(
            inspect_fixture_with_roots(
                &[("lib.rs", source)],
                "lib.rs",
                "define_i18n_module",
                &["es_fluent_manager_embedded"],
            ),
            InspectionOutcome::NotFound
        );
    }

    assert_eq!(
        inspect_fixture_with_roots(
            &[(
                "lib.rs",
                "es_fluent_manager_embedded::define_i18n_module!();"
            )],
            "lib.rs",
            "define_i18n_module",
            &[],
        ),
        InspectionOutcome::NotFound
    );

    assert!(matches!(
        inspect_fixture_with_roots(
            &[("lib.rs", "manager::define_i18n_module!();")],
            "lib.rs",
            "define_i18n_module",
            &["manager"],
        ),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn comments_and_strings_do_not_count_as_calls_or_macros() {
    assert_eq!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn main() { let _ = \"track_i18n_assets\"; } // track_i18n_assets()"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::NotFound
    );
    assert_eq!(
        inspect_fixture(
            &[(
                "lib.rs",
                "const _: &str = \"define_i18n_module!\"; /* define_i18n_module! */"
            )],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module", None)
        ),
        InspectionOutcome::NotFound
    );
}
