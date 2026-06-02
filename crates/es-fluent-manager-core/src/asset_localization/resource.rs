pub use es_fluent_shared::resource::{
    LocaleRelativeFtlPath, ModuleResourceSpec, ResourceKey, ResourcePlan, ResourcePlanError,
    locale_is_ready, optional_resource_keys_from_plan, required_resource_keys_from_plan,
    resource_plan_for, try_resource_plan_for,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use unic_langid::langid;

    #[test]
    fn resource_key_explicit_constructors_preserve_key_and_domain() {
        let dynamic = ResourceKey::try_new("demo/ui").expect("dynamic key");
        let static_key = ResourceKey::from_static_path("demo/errors");

        assert_eq!(dynamic.as_str(), "demo/ui");
        assert_eq!(dynamic.domain(), "demo");
        assert_eq!(dynamic.as_ref(), "demo/ui");
        assert_eq!(static_key.to_string(), "demo/errors");
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
            ModuleResourceSpec::new(ResourceKey::from_static_path("demo"), "demo.ftl", true),
            ModuleResourceSpec::new(
                ResourceKey::from_static_path("demo/optional"),
                "demo/optional.ftl",
                false,
            ),
        ];

        let required = required_resource_keys_from_plan(&plan);
        let optional = optional_resource_keys_from_plan(&plan);

        assert!(required.contains(&ResourceKey::from_static_path("demo")));
        assert!(!required.contains(&ResourceKey::from_static_path("demo/optional")));
        assert!(optional.contains(&ResourceKey::from_static_path("demo/optional")));
        assert!(!optional.contains(&ResourceKey::from_static_path("demo")));

        let loaded = HashSet::from([ResourceKey::from_static_path("demo")]);
        assert!(locale_is_ready(&required, &loaded));

        let unloaded = HashSet::new();
        assert!(!locale_is_ready(&required, &unloaded));
    }
}
