use super::*;
use fluent_fallback::{env::LocalesProvider, generator::BundleGenerator};
use rustc_hash::FxHashSet;
use unic_langid::langid;

fn resource(source: &str) -> Arc<FluentResource> {
    Arc::new(FluentResource::try_new(source.to_string()).expect("test FTL should parse"))
}

fn empty_resource_ids() -> FxHashSet<fluent_fallback::types::ResourceId> {
    FxHashSet::default()
}

#[test]
fn ordered_locales_returns_a_fresh_iterator_each_time() {
    let locales = OrderedLocales(vec![langid!("fr-CA"), langid!("fr"), langid!("en")]);

    assert_eq!(
        locales.locales().collect::<Vec<_>>(),
        vec![langid!("fr-CA"), langid!("fr"), langid!("en")]
    );
    assert_eq!(
        locales.locales().collect::<Vec<_>>(),
        vec![langid!("fr-CA"), langid!("fr"), langid!("en")]
    );
}

#[test]
fn static_bundle_generator_builds_success_error_and_iter_results() {
    let en = langid!("en");
    let fr = langid!("fr");
    let generator = StaticBundleGenerator::new(&[
        (en.clone(), vec![resource("hello = Hello")]),
        (
            fr.clone(),
            vec![resource("dupe = first"), resource("dupe = second")],
        ),
    ]);

    assert!(
        generator
            .build_bundle(&en)
            .expect("en should be present")
            .is_ok()
    );
    assert!(
        generator
            .build_bundle(&fr)
            .expect("fr should be present")
            .is_err()
    );
    assert!(generator.build_bundle(&langid!("de")).is_none());

    let iter_results = generator
        .bundles_iter(
            vec![langid!("de"), en.clone(), fr.clone()].into_iter(),
            empty_resource_ids(),
        )
        .collect::<Vec<_>>();
    assert_eq!(iter_results.len(), 2);
    assert!(iter_results[0].is_ok());
    assert!(iter_results[1].is_err());

    let _empty_stream = generator.bundles_stream(Vec::new().into_iter(), empty_resource_ids());
}

#[test]
fn bundle_helpers_localize_values_and_report_errors() {
    let lang = langid!("en-US");
    let first = resource(
        "hello = Hello { $name }\nneeds-name = Missing { $missing }\nattr-only =\n    .label = Label",
    );
    let duplicate = resource("hello = Duplicate");
    let mut bundle = FluentBundle::new_concurrent(vec![lang.clone()]);

    let add_errors = add_resources_to_bundle(&mut bundle, vec![first, duplicate]);
    assert!(!add_errors.is_empty());

    let mut args = HashMap::new();
    args.insert("name", FluentValue::from("Mark"));
    assert!(build_fluent_args(None).is_none());
    assert!(build_fluent_args(Some(&args)).is_some());

    let (value, errors) =
        localize_with_bundle(&bundle, "hello", Some(&args)).expect("message should be present");
    assert!(value.contains("Hello"));
    assert!(value.contains("Mark"));
    assert!(errors.is_empty());

    let (_value, errors) =
        localize_with_bundle(&bundle, "needs-name", None).expect("message should be present");
    assert!(!errors.is_empty());
    assert!(localize_with_bundle(&bundle, "missing", None).is_none());
    assert!(localize_with_bundle(&bundle, "attr-only", None).is_none());

    let (sync_bundle, sync_errors) = build_sync_bundle(&lang, vec![resource("sync = Sync")]);
    assert!(sync_errors.is_empty());
    assert_eq!(sync_bundle.locales, vec![langid!("en-US"), langid!("en")]);
}

#[test]
fn fallback_resource_localization_uses_ordered_locale_resources() {
    let mut args = HashMap::new();
    args.insert("name", FluentValue::from("Mark"));
    let locale_resources = vec![
        (langid!("en-US"), vec![resource("hello = Howdy { $name }")]),
        (langid!("en"), vec![resource("fallback-only = Fallback")]),
    ];

    let (value, errors) = localize_with_fallback_resources(&locale_resources, "hello", Some(&args));
    let value = value.expect("localized value should be present");
    assert!(value.contains("Howdy"));
    assert!(value.contains("Mark"));
    assert!(!fallback_errors_are_fatal(&errors));

    let (fallback, errors) =
        localize_with_fallback_resources(&locale_resources, "fallback-only", None);
    assert_eq!(fallback, Some("Fallback".to_string()));
    assert!(!fallback_errors_are_fatal(&errors));

    let (missing, errors) = localize_with_fallback_resources(&locale_resources, "missing", None);
    assert_eq!(missing, None);
    assert!(!fallback_errors_are_fatal(&errors));

    let (empty, errors) = localize_with_fallback_resources(&[], "hello", None);
    assert_eq!(empty, None);
    assert!(errors.is_empty());
    assert!(!fallback_errors_are_fatal(&[]));
}
