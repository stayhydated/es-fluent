use super::*;

#[test]
fn literal_includes_are_followed() {
    assert!(matches!(
        inspect_fixture(
            &[
                (
                    "build.rs",
                    "include!(\"support.rs\"); fn main() { configure(); }"
                ),
                (
                    "support.rs",
                    "use es_fluent_build::track_i18n_assets; fn configure() { track_i18n_assets(); }"
                )
            ],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn dynamic_includes_aliases_and_conditional_matches_are_indeterminate() {
    for source in [
        "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));",
        "use es_fluent_build::track_i18n_assets as track; fn main() { track(); }",
        "fn track_i18n_assets() {} fn main() { track_i18n_assets(); }",
        "mod local { pub fn track_i18n_assets() {} } fn main() { local::track_i18n_assets(); }",
        "#[cfg(feature = \"i18n\")] use es_fluent_build::track_i18n_assets; fn main() { track_i18n_assets(); }",
        "#[cfg(feature = \"i18n\")] fn configure() { track_i18n_assets(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("lib.rs", source)],
                "lib.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(_)
        ));
    }
    assert!(matches!(
        inspect_fixture(
            &[(
                "lib.rs",
                "use es_fluent_manager_embedded::define_i18n_module as define; define!();"
            )],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module", None)
        ),
        InspectionOutcome::Indeterminate(_)
    ));
    assert!(matches!(
        inspect_fixture(
            &[(
                "lib.rs",
                "macro_rules! define_i18n_module { () => {} } define_i18n_module!();"
            )],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module", None)
        ),
        InspectionOutcome::Indeterminate(_)
    ));
    assert!(matches!(
        inspect_fixture(
            &[(
                "lib.rs",
                "mod imported { use es_fluent_manager_embedded::define_i18n_module; } define_i18n_module!();"
            )],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module", None)
        ),
        InspectionOutcome::Indeterminate(_)
    ));
    assert!(matches!(
        inspect_fixture(
            &[(
                "lib.rs",
                "fn setup() { #[cfg(feature = \"i18n\")] define_i18n_module!(); }"
            )],
            "lib.rs",
            SourceTarget::Macro("define_i18n_module", None)
        ),
        InspectionOutcome::Indeterminate(_)
    ));
    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "#[cfg(feature = \"other\")] fn conditional() { track_i18n_assets(); } fn main() { es_fluent_build::track_i18n_assets(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn opaque_item_macro_expansions_are_indeterminate() {
    assert!(matches!(
        inspect_fixture(
            &[("build.rs", "configure_i18n!(); fn main() {}")],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("opaque item macro expansion")
    ));
}

#[test]
fn verified_calls_with_opaque_item_macro_expansions_are_indeterminate() {
    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                r#"macro_rules! define_local_helper {
    () => {
        mod es_fluent_build {
            pub fn track_i18n_assets() {}
        }
    };
}
define_local_helper!();
fn main() { es_fluent_build::track_i18n_assets(); }
"#
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("opaque item macro expansion")
    ));
}

#[test]
fn opaque_statement_macro_expansions_are_indeterminate() {
    assert!(matches!(
        inspect_fixture(
            &[("build.rs", "fn main() { configure_i18n!(); }")],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("opaque statement macro expansion")
    ));
}

#[test]
fn build_helper_calls_after_opaque_macros_are_indeterminate() {
    for source in [
        "fn main() { panic!(\"stop\"); es_fluent_build::track_i18n_assets(); }",
        "fn main() { configure_i18n!(); es_fluent_build::track_i18n_assets(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("under control flow that could not be proven to execute")
        ));
    }
}

#[test]
fn opaque_helper_references_are_indeterminate() {
    for source in [
        "use es_fluent_build::track_i18n_assets; fn main() { let f: fn() = track_i18n_assets; f(); }",
        "fn main() { let f: fn() = es_fluent_build::track_i18n_assets; f(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("opaque reference to `track_i18n_assets`")
        ));
    }
}

#[test]
fn opaque_expression_macro_expansions_are_indeterminate() {
    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn main() { let _configuration = configure_i18n!(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("opaque expression macro expansion")
    ));
}

#[test]
fn source_graph_marks_macro_wrapped_include_indeterminate_without_a_doctor_target() {
    let temp = tempfile::tempdir().expect("tempdir");
    let support = temp.path().join("support");
    fs::create_dir_all(&support).expect("create support directory");
    fs::write(
            temp.path().join("build.rs"),
            "macro_rules! load_config { () => { include!(\"support/config.rs\"); }; } load_config!(); fn main() {}\n",
        )
        .expect("write build target");
    fs::write(support.join("config.rs"), "pub fn configure() {}\n").expect("write included source");

    let graph = reachable_source_graph(&temp.path().join("build.rs"), temp.path());

    assert!(
        graph
            .indeterminate_reasons
            .iter()
            .any(|reason| { reason.contains("macro wrapper") && reason.contains("include") })
    );
    assert!(
        graph
            .indeterminate_reasons
            .iter()
            .any(|reason| reason.contains("opaque item macro expansion"))
    );
}
