use crate::FluentValue;
use es_fluent_manager_core::FluentManager;
use std::collections::HashMap;
use std::sync::Arc;

/// A typed Fluent message that can be resolved by an explicit localization
/// backend.
///
/// Derive macros implement this trait for `#[derive(EsFluent)]` and generated
/// variant enums. Runtime managers use it to keep typed message call sites while
/// routing lookup through a request, component, or application-scoped manager.
pub trait FluentMessage {
    /// Converts the message into a localized string using the supplied lookup
    /// callback.
    fn to_fluent_string_with(
        &self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String;
}

impl<T: FluentMessage + ?Sized> FluentMessage for &T {
    fn to_fluent_string_with(
        &self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        (**self).to_fluent_string_with(localize)
    }
}

/// Runtime context that can resolve Fluent message IDs.
///
/// This is the manager-facing replacement for the removed context-free global
/// lookup. Managers and framework adapters implement this trait so callers
/// can keep typed message values while passing the active localization context
/// explicitly.
pub trait FluentLocalizer {
    /// Localizes a message by ID using the localizer's default lookup behavior.
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String>;

    /// Localizes a message by ID within a specific domain.
    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String>;
}

impl FluentLocalizer for FluentManager {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        FluentManager::localize(self, id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        FluentManager::localize_in_domain(self, domain, id, args)
    }
}

impl<T: FluentLocalizer + ?Sized> FluentLocalizer for &T {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (**self).localize(id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (**self).localize_in_domain(domain, id, args)
    }
}

impl<T: FluentLocalizer + ?Sized> FluentLocalizer for Arc<T> {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (**self).localize(id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (**self).localize_in_domain(domain, id, args)
    }
}

/// Convenience methods for explicit localization contexts.
pub trait FluentLocalizerExt: FluentLocalizer {
    /// Localizes a message by ID, falling back to the ID and logging a warning
    /// when the message is missing.
    fn localize_or_id<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        self.localize(id, args).unwrap_or_else(|| {
            tracing::warn!(message_id = id, "missing Fluent message");
            id.to_string()
        })
    }

    /// Localizes a domain-scoped message by ID, falling back to the ID and
    /// logging a warning when the message is missing.
    fn localize_in_domain_or_id<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        self.localize_in_domain(domain, id, args)
            .unwrap_or_else(|| {
                tracing::warn!(domain, message_id = id, "missing Fluent message");
                id.to_string()
            })
    }

    /// Renders a derived typed message through this explicit localizer.
    fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        message.to_fluent_string_with(&mut |domain, id, args| {
            self.localize_in_domain_or_id(domain, id, args)
        })
    }

    /// Renders a derived typed message through this explicit localizer without
    /// logging missing-message warnings.
    fn localize_message_silent<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        message.to_fluent_string_with(&mut |domain, id, args| {
            self.localize_in_domain(domain, id, args)
                .unwrap_or_else(|| id.to_string())
        })
    }
}

impl<T: FluentLocalizer + ?Sized> FluentLocalizerExt for T {}

#[doc(hidden)]
pub trait IntoFluentValue<'a> {
    fn into_fluent_value(self) -> FluentValue<'a>;
}

impl<'a, T> IntoFluentValue<'a> for T
where
    T: Into<FluentValue<'a>>,
{
    fn into_fluent_value(self) -> FluentValue<'a> {
        self.into()
    }
}

/// Wrapper used by generated `FluentMessage` implementations to keep nested
/// localized arguments on the same explicit lookup path as the outer message.
#[doc(hidden)]
pub struct FluentArgumentValue<T> {
    value: T,
}

impl<T> FluentArgumentValue<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

/// Converts generated message arguments into Fluent values.
///
/// This intentionally uses autoref-priority implementations: exact
/// `FluentArgumentValue<T>` dispatch is selected for nested `FluentMessage`
/// values, while ordinary argument values fall back to `Into<FluentValue>` via
/// `&FluentArgumentValue<T>`.
#[doc(hidden)]
pub trait IntoFluentArgumentValue<'a> {
    fn into_fluent_argument_value(
        self,
        localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a>;
}

impl<'a, T> IntoFluentArgumentValue<'a> for FluentArgumentValue<T>
where
    T: FluentMessage,
{
    fn into_fluent_argument_value(
        self,
        localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        self.value.to_fluent_string_with(localize).into()
    }
}

impl<'a, T> IntoFluentArgumentValue<'a> for &FluentArgumentValue<T>
where
    T: Clone + IntoFluentValue<'a>,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        self.value.clone().into_fluent_value()
    }
}

impl<'a> IntoFluentArgumentValue<'a> for FluentArgumentValue<bool> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        if self.value { "true" } else { "false" }.into()
    }
}

impl<'a, T> IntoFluentArgumentValue<'a> for FluentArgumentValue<Option<T>>
where
    T: Clone + IntoFluentValue<'a>,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        match self.value {
            Some(value) => value.into_fluent_value(),
            None => FluentValue::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panic_lookup<'a>(
        _domain: &str,
        _id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        panic!("ordinary arguments should not invoke nested localization")
    }

    fn assert_string(value: FluentValue<'_>, expected: &str) {
        match value {
            FluentValue::String(value) => assert_eq!(value.as_ref(), expected),
            other => panic!("expected string FluentValue, got {other:?}"),
        }
    }

    fn assert_number(value: FluentValue<'_>, expected: f64) {
        match value {
            FluentValue::Number(value) => assert_eq!(value.value, expected),
            other => panic!("expected number FluentValue, got {other:?}"),
        }
    }

    #[test]
    fn argument_conversion_handles_primitive_values() {
        let mut localize = panic_lookup;

        let string_value =
            FluentArgumentValue::new("borrowed").into_fluent_argument_value(&mut localize);
        assert_string(string_value, "borrowed");

        let number_value =
            FluentArgumentValue::new(42i32).into_fluent_argument_value(&mut localize);
        assert_number(number_value, 42.0);

        let bool_value = FluentArgumentValue::new(true).into_fluent_argument_value(&mut localize);
        assert_string(bool_value, "true");
    }

    #[test]
    fn argument_conversion_handles_optional_and_missing_values() {
        let mut localize = panic_lookup;

        let optional_value =
            FluentArgumentValue::new(Some("optional")).into_fluent_argument_value(&mut localize);
        assert_string(optional_value, "optional");

        let missing_value = FluentArgumentValue::new(Option::<String>::None)
            .into_fluent_argument_value(&mut localize);
        assert!(matches!(missing_value, FluentValue::None));
    }

    #[test]
    fn argument_conversion_handles_borrowed_and_owned_values() {
        let mut localize = panic_lookup;
        let borrowed = String::from("borrowed string");

        let borrowed_value =
            FluentArgumentValue::new(&borrowed).into_fluent_argument_value(&mut localize);
        assert_string(borrowed_value, "borrowed string");

        let owned_value = FluentArgumentValue::new(String::from("owned string"))
            .into_fluent_argument_value(&mut localize);
        assert_string(owned_value, "owned string");
    }

    #[derive(Clone)]
    struct NestedMessage;

    impl FluentMessage for NestedMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            localize("nested-domain", "nested-id", None)
        }
    }

    #[test]
    fn argument_conversion_localizes_nested_messages_with_current_callback() {
        let mut localize =
            |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
                assert_eq!(domain, "nested-domain");
                assert_eq!(id, "nested-id");
                assert!(args.is_none());
                "nested value".to_string()
            };

        let value =
            FluentArgumentValue::new(NestedMessage).into_fluent_argument_value(&mut localize);
        assert_string(value, "nested value");
    }

    struct StaticLocalizer {
        value: &'static str,
    }

    impl FluentLocalizer for StaticLocalizer {
        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            if id == "nested-id" {
                Some(self.value.to_string())
            } else {
                None
            }
        }

        fn localize_in_domain<'a>(
            &self,
            domain: &str,
            id: &str,
            args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            if domain == "nested-domain" {
                self.localize(id, args)
            } else {
                None
            }
        }
    }

    #[test]
    fn localize_message_uses_the_explicit_localizer() {
        let en = StaticLocalizer { value: "Hello" };
        let fr = StaticLocalizer { value: "Bonjour" };

        assert_eq!(en.localize_message(&NestedMessage), "Hello");
        assert_eq!(fr.localize_message(&NestedMessage), "Bonjour");
        assert_eq!(en.localize_message(&NestedMessage), "Hello");
    }
}
