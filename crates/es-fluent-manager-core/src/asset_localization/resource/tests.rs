use super::*;
use unic_langid::langid;

#[test]
fn resource_key_conversions_preserve_key_and_domain() {
    let from_string: ResourceKey = "demo/ui".to_string().into();
    let from_str: ResourceKey = "demo/errors".into();

    assert_eq!(from_string.as_str(), "demo/ui");
    assert_eq!(from_string.domain(), "demo");
    assert_eq!(from_string.as_ref(), "demo/ui");
    assert_eq!(from_str.to_string(), "demo/errors");
}

#[test]
fn resource_plan_for_handles_base_and_namespaced_resources() {
    let base_plan = resource_plan_for("demo", &[]);
    assert_eq!(base_plan.len(), 1);
    assert_eq!(base_plan[0].key.as_str(), "demo");
    assert_eq!(base_plan[0].locale_relative_path, "demo.ftl");
    assert_eq!(
        base_plan[0].locale_path(&langid!("en-US")),
        "en-US/demo.ftl"
    );
    assert!(base_plan[0].required);

    let namespaced_plan = resource_plan_for("demo", &["ui", "ui", "errors"]);
    let keys: Vec<_> = namespaced_plan
        .iter()
        .map(|spec| spec.key.as_str())
        .collect();
    assert_eq!(keys, vec!["demo", "demo/ui", "demo/errors"]);
    assert!(!namespaced_plan[0].required);
    assert_eq!(namespaced_plan[0].locale_relative_path, "demo.ftl");
    assert!(namespaced_plan[1].required);
    assert!(
        namespaced_plan
            .iter()
            .all(|spec| spec.locale_relative_path.ends_with(".ftl"))
    );
}

#[test]
#[should_panic(expected = "resource_plan_for received invalid namespace")]
fn resource_plan_for_rejects_invalid_namespaces() {
    let _ = resource_plan_for("demo", &["../outside"]);
}

#[test]
fn required_and_optional_keys_reflect_plan_membership() {
    let plan = vec![
        module_resource_spec("demo", "demo.ftl", true),
        module_resource_spec("demo/optional", "demo/optional.ftl", false),
    ];

    let required = required_resource_keys_from_plan(&plan);
    let optional = optional_resource_keys_from_plan(&plan);

    assert!(required.contains(&ResourceKey::from("demo")));
    assert!(!required.contains(&ResourceKey::from("demo/optional")));
    assert!(optional.contains(&ResourceKey::from("demo/optional")));
    assert!(!optional.contains(&ResourceKey::from("demo")));

    let loaded = HashSet::from([ResourceKey::from("demo")]);
    assert!(locale_is_ready(&required, &loaded));

    let unloaded = HashSet::new();
    assert!(!locale_is_ready(&required, &unloaded));
}
