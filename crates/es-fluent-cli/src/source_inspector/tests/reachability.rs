use super::*;

#[test]
fn build_helper_calls_must_be_reachable_from_main() {
    for source in [
        "fn unused() { es_fluent_build::track_i18n_assets(); } fn main() {}",
        "fn unused() { fn main() { es_fluent_build::track_i18n_assets(); } } fn main() {}",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("could not be proven reachable")
        ));
    }

    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn configure() { es_fluent_build::track_i18n_assets(); } fn main() { configure(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Found(_)
    ));

    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "mod helper { pub fn configure() { es_fluent_build::track_i18n_assets(); } } fn configure() {} fn main() { configure(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("could not be proven reachable")
    ));
}

#[test]
fn block_local_wrapper_shadowing_does_not_make_outer_helper_reachable() {
    let outcome = inspect_fixture(
        &[(
            "build.rs",
            "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { fn setup() {} setup(); }",
        )],
        "build.rs",
        SourceTarget::Call("track_i18n_assets"),
    );

    assert!(matches!(outcome, InspectionOutcome::Indeterminate(_)));
}

#[test]
fn local_value_bindings_do_not_make_outer_helpers_reachable() {
    for source in [
        "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { let setup = || {}; setup(); }",
        "fn setup() { es_fluent_build::track_i18n_assets(); } fn run(setup: impl Fn()) { setup(); } fn main() { run(|| {}); }",
        "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { let (setup, _) = (|| {}, 0); setup(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets"),
            ),
            InspectionOutcome::Indeterminate(_)
        ));
    }
}

#[test]
fn control_flow_pattern_bindings_shadow_imported_helpers_lexically() {
    let imported_helper = "mod helper { pub fn setup() { es_fluent_build::track_i18n_assets(); } } use helper::setup;";
    for body in [
        "fn main() { for setup in [|| {}] { setup(); } }",
        "fn main() { match Some(|| {}) { Some(setup) if { setup(); true } => setup(), _ => {} } }",
        "fn main() { if let Some(setup) = Some(|| {}) { setup(); } }",
        "fn main() { while let Some(setup) = Some(|| {}) { setup(); break; } }",
        "fn main() { if let Some(setup) = Some(|| {}) && { setup(); true } { setup(); } }",
        "fn main() { while let Some(setup) = Some(|| {}) && { setup(); true } { setup(); break; } }",
    ] {
        let source = format!("{imported_helper} {body}");
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", &source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets"),
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("could not be proven reachable")
        ));
    }

    for body in [
        "fn main() { for setup in [setup()] { let _ = setup; } }",
        "fn main() { match setup() { setup => { let _ = setup; } } }",
        "fn main() { if let Some(setup) = setup() { let _ = setup; } }",
    ] {
        let source = format!("{imported_helper} {body}");
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", &source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets"),
            ),
            InspectionOutcome::Found(_)
        ));
    }

    let else_body = "fn main() { if let Some(setup) = Some(|| {}) { setup(); } else { setup(); } }";
    let source = format!("{imported_helper} {else_body}");
    assert!(matches!(
        inspect_fixture(
            &[("build.rs", &source)],
            "build.rs",
            SourceTarget::Call("track_i18n_assets"),
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("control flow")
    ));
}

#[test]
fn unsupported_block_imports_do_not_make_outer_helpers_reachable() {
    for import in [
        "use ::external_crate::setup;",
        "#[cfg(feature = \"external\")] use external_crate::setup;",
        "use external_crate::*;",
    ] {
        let source = format!(
            "fn setup() {{ es_fluent_build::track_i18n_assets(); }} fn main() {{ {import} setup(); }}"
        );
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", &source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets"),
            ),
            InspectionOutcome::Indeterminate(_)
        ));
    }
}

#[test]
fn block_local_callable_items_do_not_make_outer_helpers_reachable() {
    for local_item in [
        "const setup: fn() = noop;",
        "static setup: fn() = noop;",
        "struct setup();",
    ] {
        let source = format!(
            "fn setup() {{ es_fluent_build::track_i18n_assets(); }} fn noop() {{}} fn main() {{ {local_item} setup(); }}"
        );
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", &source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets"),
            ),
            InspectionOutcome::Indeterminate(_)
        ));
    }
}

#[test]
fn branch_guarded_build_helper_calls_are_indeterminate() {
    for source in [
        "fn main() { if false { es_fluent_build::track_i18n_assets(); } }",
        "fn main() { match false { true => es_fluent_build::track_i18n_assets(), false => {} } }",
        "fn main() { while false { es_fluent_build::track_i18n_assets(); } }",
        "fn main() { false && { es_fluent_build::track_i18n_assets(); true }; }",
        "fn main() { let _future = async { es_fluent_build::track_i18n_assets(); }; }",
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
fn conditionally_reached_build_helper_functions_are_indeterminate() {
    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { if false { setup(); } }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("under control flow that could not be proven to execute")
    ));

    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { if false { setup(); } setup(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn build_helper_calls_after_conditional_exits_are_indeterminate() {
    for source in [
        "fn skip() -> bool { false } fn main() { if skip() { return; } es_fluent_build::track_i18n_assets(); }",
        "fn skip() -> bool { false } fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { if skip() { return; } setup(); }",
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
fn build_helper_calls_after_diverging_calls_are_indeterminate() {
    for source in [
        "fn main() { std::process::exit(0); es_fluent_build::track_i18n_assets(); }",
        "fn stop() -> ! { loop {} } fn main() { stop(); es_fluent_build::track_i18n_assets(); }",
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
fn block_local_function_shadowing_build_helper_import_is_indeterminate() {
    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "use es_fluent_build::track_i18n_assets; fn main() { fn track_i18n_assets() {} track_i18n_assets(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("could not be resolved to the expected es-fluent dependency")
    ));
}

#[test]
fn local_binding_shadowing_build_helper_import_is_indeterminate() {
    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "use es_fluent_build::track_i18n_assets; fn main() { let track_i18n_assets = || {}; track_i18n_assets(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("could not be resolved to the expected es-fluent dependency")
    ));

    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "use es_fluent_build::track_i18n_assets; fn main() { let track_i18n_assets = { track_i18n_assets(); || {} }; }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn build_helper_calls_after_return_are_not_found() {
    assert_eq!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn main() { return; es_fluent_build::track_i18n_assets(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::NotFound
    );
}

#[test]
fn build_helper_calls_after_nested_return_are_not_found() {
    assert_eq!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn main() { { return; } es_fluent_build::track_i18n_assets(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::NotFound
    );
}

#[test]
fn build_helper_calls_after_diverging_loops_do_not_pass() {
    assert_eq!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn main() { loop {} es_fluent_build::track_i18n_assets(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::NotFound
    );

    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn setup() { es_fluent_build::track_i18n_assets(); } fn main() { loop { continue; } setup(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Indeterminate(reason)
            if reason.contains("could not be proven reachable")
    ));
}

#[test]
fn build_helper_calls_after_wrapped_diverging_loops_do_not_pass() {
    for source in [
        "fn main() { let _never = loop {}; es_fluent_build::track_i18n_assets(); }",
        "fn main() { let mut value = (); value = { loop {} }; es_fluent_build::track_i18n_assets(); }",
    ] {
        assert_eq!(
            inspect_fixture(
                &[("build.rs", source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::NotFound
        );
    }
}

#[test]
fn loops_in_deferred_or_item_bodies_do_not_hide_following_build_helpers() {
    assert!(matches!(
        inspect_fixture(
            &[(
                "build.rs",
                "fn main() { let _closure = || loop {}; let _future = async { loop {} }; fn stop() { loop {} } es_fluent_build::track_i18n_assets(); }"
            )],
            "build.rs",
            SourceTarget::Call("track_i18n_assets")
        ),
        InspectionOutcome::Found(_)
    ));
}

#[test]
fn build_helper_calls_after_loops_with_breaks_are_found() {
    for source in [
        "fn main() { loop { break; } es_fluent_build::track_i18n_assets(); }",
        "fn main() { let _value = loop { break; }; es_fluent_build::track_i18n_assets(); }",
        "fn main() { let mut value = (); value = loop { break; }; es_fluent_build::track_i18n_assets(); }",
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
fn build_helper_calls_after_conditionally_breaking_loops_are_indeterminate() {
    for source in [
        "fn main() { loop { if runtime_condition() { break; } } es_fluent_build::track_i18n_assets(); }",
        "fn main() { loop { if runtime_condition() { continue; } break; } es_fluent_build::track_i18n_assets(); }",
        "fn main() { let _value = loop { if runtime_condition() { break; } }; es_fluent_build::track_i18n_assets(); }",
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
fn local_module_shadowing_build_dependency_is_indeterminate() {
    for source in [
        "mod es_fluent_build { pub fn track_i18n_assets() {} } fn main() { es_fluent_build::track_i18n_assets(); }",
        "mod local { pub mod es_fluent_build { pub fn track_i18n_assets() {} } } use local::es_fluent_build; fn main() { es_fluent_build::track_i18n_assets(); }",
        "mod local { pub fn track_i18n_assets() {} } use local as es_fluent_build; fn main() { es_fluent_build::track_i18n_assets(); }",
        "extern crate self as es_fluent_build; fn track_i18n_assets() {} fn main() { es_fluent_build::track_i18n_assets(); }",
    ] {
        assert!(matches!(
            inspect_fixture(
                &[("build.rs", source)],
                "build.rs",
                SourceTarget::Call("track_i18n_assets")
            ),
            InspectionOutcome::Indeterminate(reason)
                if reason.contains("could not be resolved to the expected es-fluent dependency")
        ));
    }
}
