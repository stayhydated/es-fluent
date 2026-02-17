use fluent_bundle::FluentResource;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// A static Fluent resource that can be injected directly into localization bundles.
pub trait StaticI18nResource: Send + Sync {
    /// Domain under which the resource should be registered.
    fn domain(&self) -> &'static str;

    /// Returns the resource content.
    fn resource(&self) -> Arc<FluentResource>;

    /// Returns `true` when this resource should be included for the provided language.
    fn matches_language(&self, _lang: &LanguageIdentifier) -> bool {
        true
    }
}

inventory::collect!(&'static dyn StaticI18nResource);

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    struct TestStaticResource;

    impl StaticI18nResource for TestStaticResource {
        fn domain(&self) -> &'static str {
            "test-domain"
        }

        fn resource(&self) -> Arc<FluentResource> {
            Arc::new(FluentResource::try_new("hello = hi".to_string()).expect("valid ftl"))
        }
    }

    #[test]
    fn static_resource_defaults_to_matching_all_languages() {
        let resource = TestStaticResource;
        assert_eq!(resource.domain(), "test-domain");
        assert!(resource.matches_language(&langid!("en-US")));
        assert!(resource.resource().get_entry(0).is_some());
    }
}
