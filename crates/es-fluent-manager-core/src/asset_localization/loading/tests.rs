use super::*;
use unic_langid::langid;

fn spec(key: &str, required: bool) -> ModuleResourceSpec {
    ModuleResourceSpec {
        key: ResourceKey::new(key),
        locale_relative_path: format!("{key}.ftl"),
        required,
    }
}

#[test]
fn load_locale_resources_records_loaded_missing_and_error_states() {
    let required = spec("app", true);
    let optional_missing = spec("app/optional", false);
    let optional_error = spec("app/broken", false);
    let plan = vec![
        required.clone(),
        optional_missing.clone(),
        optional_error.clone(),
    ];
    let loaded = Arc::new(
        FluentResource::try_new("hello = Hello".to_string()).expect("valid fluent resource"),
    );

    let (resources, report) = load_locale_resources(&plan, |candidate| {
        if candidate.key == required.key {
            ResourceLoadStatus::Loaded(loaded.clone())
        } else if candidate.key == optional_missing.key {
            ResourceLoadStatus::Missing
        } else {
            ResourceLoadStatus::Error(ResourceLoadError::load(candidate, "failed"))
        }
    });

    assert_eq!(resources.len(), 1);
    assert!(report.required_keys().contains(&required.key));
    assert!(report.optional_keys().contains(&optional_missing.key));
    assert!(report.loaded_keys().contains(&required.key));
    assert_eq!(report.errors().len(), 2);
    assert!(report.is_ready());
}

#[test]
fn locale_load_report_required_errors_make_locale_unready() {
    let required = spec("app", true);
    let mut report = LocaleLoadReport::from_plan(std::slice::from_ref(&required));

    report.mark_loaded(required.key.clone());
    assert!(report.is_ready());

    report.record_error(ResourceLoadError::missing(&required));
    assert!(report.has_required_errors());
    assert!(report.missing_required_keys().contains(&required.key));
    assert!(!report.is_ready());
}

#[test]
fn resource_load_error_display_messages_cover_all_failure_kinds() {
    let required = spec("app", true);
    let optional = spec("optional", false);

    let missing = ResourceLoadError::missing(&optional);
    assert_eq!(
        missing.to_string(),
        "missing optional resource 'optional' at 'optional.ftl'"
    );
    assert!(!missing.is_required());

    let invalid_utf8 =
        parse_fluent_resource_bytes(&required, &[0xff]).expect_err("invalid UTF-8 should fail");
    assert!(
        invalid_utf8
            .to_string()
            .contains("invalid UTF-8 in required resource 'app'")
    );
    assert!(invalid_utf8.is_required());

    let parse = parse_fluent_resource_content(&optional, "hello = {".to_string())
        .expect_err("invalid Fluent should fail");
    assert!(
        parse
            .to_string()
            .contains("failed to parse optional resource 'optional'")
    );

    let load = ResourceLoadError::load(&required, "asset server failed");
    assert_eq!(
        load.to_string(),
        "failed to load required resource 'app' at 'app.ftl': asset server failed"
    );
    assert_eq!(load.key(), &required.key);
}

#[test]
fn build_locale_load_report_filters_by_language_and_loaded_state() {
    let en = langid!("en");
    let fr = langid!("fr");
    let app = spec("app", true);
    let optional = spec("optional", false);
    let other = spec("other", true);
    let resource = Arc::new(
        FluentResource::try_new("hello = Hello".to_string()).expect("valid fluent resource"),
    );
    let resource_specs = HashMap::from([
        ((en.clone(), app.key.clone()), app.clone()),
        ((en.clone(), optional.key.clone()), optional.clone()),
        ((fr.clone(), other.key.clone()), other.clone()),
    ]);
    let loaded_resources = HashMap::from([((en.clone(), app.key.clone()), resource)]);
    let load_errors = HashMap::from([(
        (en.clone(), optional.key.clone()),
        ResourceLoadError::missing(&optional),
    )]);

    let report = build_locale_load_report(&resource_specs, &loaded_resources, &load_errors, &en);

    assert!(report.required_keys().contains(&app.key));
    assert!(!report.required_keys().contains(&other.key));
    assert!(report.optional_keys().contains(&optional.key));
    assert!(report.loaded_keys().contains(&app.key));
    assert_eq!(report.errors().len(), 1);
    assert!(report.is_ready());
}

#[test]
fn parse_fluent_resource_bytes_reports_invalid_utf8_with_resource_metadata() {
    let resource_spec = spec("app", true);
    let err = parse_fluent_resource_bytes(&resource_spec, &[0xff])
        .expect_err("invalid UTF-8 should be reported");

    assert!(matches!(
        err,
        ResourceLoadError::InvalidUtf8 {
            key,
            path,
            required: true,
            ..
        } if key == resource_spec.key && path == resource_spec.locale_relative_path
    ));
}

#[test]
fn persistent_locale_resource_state_helpers_update_loaded_and_error_maps() {
    let lang = langid!("en");
    let resource_spec = spec("app", true);
    let mut loaded_resources = HashMap::new();
    let mut load_errors = HashMap::new();

    parse_and_store_locale_resource_content(
        &mut loaded_resources,
        &mut load_errors,
        &lang,
        &resource_spec,
        "hello = Hello".to_string(),
    )
    .expect("valid resource should store");
    assert!(loaded_resources.contains_key(&(lang.clone(), resource_spec.key.clone())));
    assert!(load_errors.is_empty());

    let parse_err = parse_fluent_resource_content(&resource_spec, "hello = {".to_string())
        .expect_err("invalid resource should report parse error");
    record_locale_resource_error(&mut loaded_resources, &mut load_errors, &lang, parse_err);
    assert!(!loaded_resources.contains_key(&(lang.clone(), resource_spec.key.clone())));
    assert!(load_errors.contains_key(&(lang.clone(), resource_spec.key.clone())));

    clear_locale_resource(
        &mut loaded_resources,
        &mut load_errors,
        &lang,
        &resource_spec.key,
    );
    assert!(loaded_resources.is_empty());
    assert!(load_errors.is_empty());
}
