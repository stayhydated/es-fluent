use crate::FluentValue;
use es_fluent_manager_core::FluentManager;
use std::collections::HashMap;
use std::sync::Arc;

const WITH_LOOKUP_CALLBACK_COUNT_ERROR: &str =
    "FluentLocalizer::with_lookup must invoke its callback exactly once";

/// A typed Fluent message that can be resolved by an explicit localization
/// backend.
///
/// Derive macros implement this trait for `#[derive(EsFluent)]` and generated
/// variant enums. Runtime managers use it to keep typed message call sites while
/// routing lookup through a request, component, or application-scoped manager.
pub trait FluentMessage {
    /// Converts the message into a localized string using the supplied lookup
    /// callback.
    ///
    /// Manual implementations should treat `localize` as the only lookup path
    /// during rendering. Do not re-enter the same localizer to select a
    /// language or perform other lock-taking lookups from this method; managers
    /// may hold snapshot locks while invoking it.
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
///
/// # Implementing `FluentLocalizer`
///
/// Custom localizers should either use the default [`Self::with_lookup`]
/// implementation or override it to provide one render-scoped snapshot. If
/// `with_lookup(...)` is overridden, it must invoke the callback exactly once
/// before returning. Failing to do so is a logic error and will panic in
/// [`FluentLocalizerExt::localize_message`] and
/// [`FluentLocalizerExt::try_localize_message`].
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

    /// Runs a group of lookups against one render-scoped localization view.
    ///
    /// Implementations must invoke the callback exactly once, must not call it
    /// after `with_lookup(...)` returns, and should provide a stable lookup
    /// snapshot for the duration of that callback. The extension methods rely
    /// on this contract when rendering nested typed messages.
    ///
    /// The callback is the only supported lookup path inside a typed message
    /// render. Custom `FluentMessage` implementations must not re-enter the
    /// same localizer for language selection or other lock-taking operations
    /// while this callback is active.
    ///
    /// The default implementation delegates each lookup independently. Managers
    /// with mutable language selection should override this to hold the relevant
    /// lock or snapshot for the whole callback.
    ///
    /// # Example
    ///
    /// ```
    /// # use es_fluent::{FluentLocalizer, FluentValue};
    /// # use std::collections::HashMap;
    /// struct MyLocalizer;
    ///
    /// impl MyLocalizer {
    ///     fn lookup<'a>(
    ///         &self,
    ///         domain: &str,
    ///         id: &str,
    ///         _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ///     ) -> Option<String> {
    ///         Some(format!("{domain}:{id}"))
    ///     }
    /// }
    ///
    /// impl FluentLocalizer for MyLocalizer {
    ///     fn localize<'a>(
    ///         &self,
    ///         id: &str,
    ///         args: Option<&HashMap<&str, FluentValue<'a>>>,
    ///     ) -> Option<String> {
    ///         self.localize_in_domain(env!("CARGO_PKG_NAME"), id, args)
    ///     }
    ///
    ///     fn localize_in_domain<'a>(
    ///         &self,
    ///         domain: &str,
    ///         id: &str,
    ///         args: Option<&HashMap<&str, FluentValue<'a>>>,
    ///     ) -> Option<String> {
    ///         self.lookup(domain, id, args)
    ///     }
    ///
    ///     fn with_lookup(
    ///         &self,
    ///         f: &mut dyn FnMut(
    ///             &mut dyn for<'a> FnMut(
    ///                 &str,
    ///                 &str,
    ///                 Option<&HashMap<&str, FluentValue<'a>>>,
    ///             ) -> Option<String>,
    ///         ),
    ///     ) {
    ///         let mut lookup =
    ///             |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
    ///                 self.localize_in_domain(domain, id, args)
    ///             };
    ///         f(&mut lookup);
    ///     }
    /// }
    /// ```
    fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        let mut lookup = |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            self.localize_in_domain(domain, id, args)
        };
        f(&mut lookup);
    }
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

    fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        FluentManager::with_lookup(self, f);
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

    fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        (**self).with_lookup(f);
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

    fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        (**self).with_lookup(f);
    }
}

/// Public extension methods for generic explicit localization contexts.
///
/// Concrete manager crates expose inherent `localize_message(...)` methods for
/// application code. Import this trait when integration code works with a
/// generic [`FluentLocalizer`] and still needs typed message rendering.
pub trait FluentLocalizerExt: FluentLocalizer {
    /// Attempts to render a derived typed message through this explicit
    /// localizer.
    ///
    /// Returns `None` if any lookup in the message tree is missing. Use
    /// `localize_message(...)` when a message ID fallback is
    /// desired instead.
    fn try_localize_message<T>(&self, message: &T) -> Option<String>
    where
        T: FluentMessage + ?Sized,
    {
        let mut missing = false;
        let mut value = None;
        let mut callback_invocations = 0;

        self.with_lookup(&mut |lookup| {
            assert!(
                callback_invocations == 0,
                "{}",
                WITH_LOOKUP_CALLBACK_COUNT_ERROR
            );
            callback_invocations = 1;

            value = Some(message.to_fluent_string_with(&mut |domain, id, args| {
                lookup(domain, id, args).unwrap_or_else(|| {
                    missing = true;
                    String::new()
                })
            }));
        });

        assert!(
            callback_invocations == 1,
            "{}",
            WITH_LOOKUP_CALLBACK_COUNT_ERROR
        );
        let value = value.expect(WITH_LOOKUP_CALLBACK_COUNT_ERROR);
        if missing { None } else { Some(value) }
    }

    /// Renders a derived typed message through this explicit localizer.
    fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        let mut value = None;
        let mut callback_invocations = 0;

        self.with_lookup(&mut |lookup| {
            assert!(
                callback_invocations == 0,
                "{}",
                WITH_LOOKUP_CALLBACK_COUNT_ERROR
            );
            callback_invocations = 1;

            value = Some(message.to_fluent_string_with(&mut |domain, id, args| {
                lookup(domain, id, args).unwrap_or_else(|| {
                    tracing::warn!(domain, message_id = id, "missing Fluent message");
                    id.to_string()
                })
            }));
        });

        assert!(
            callback_invocations == 1,
            "{}",
            WITH_LOOKUP_CALLBACK_COUNT_ERROR
        );
        value.expect(WITH_LOOKUP_CALLBACK_COUNT_ERROR)
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

/// Borrowed wrapper used by generated `FluentMessage` implementations for
/// ordinary fields. Nested messages are rendered through the current callback;
/// scalar values are cloned only at the final conversion boundary.
#[doc(hidden)]
pub struct FluentBorrowedArgumentValue<'a, T: ?Sized> {
    value: &'a T,
}

impl<'a, T: ?Sized> FluentBorrowedArgumentValue<'a, T> {
    pub fn new(value: &'a T) -> Self {
        Self { value }
    }
}

/// Optional wrapper used by generated `FluentMessage` implementations so
/// `Option<T>` can represent missing Fluent arguments without losing nested
/// message localization for `Some(T)`.
#[doc(hidden)]
pub struct FluentOptionalArgumentValue<T> {
    value: Option<T>,
}

impl<T> FluentOptionalArgumentValue<T> {
    pub fn new(value: Option<T>) -> Self {
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

impl<'a, 'value, T> IntoFluentArgumentValue<'a> for FluentBorrowedArgumentValue<'value, T>
where
    T: FluentMessage + ?Sized,
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

impl<'a, 'value, T> IntoFluentArgumentValue<'a> for &FluentBorrowedArgumentValue<'value, T>
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
        (*self.value).clone().into_fluent_value()
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
        bool_fluent_value(self.value)
    }
}

impl<'a, 'value> IntoFluentArgumentValue<'a> for FluentBorrowedArgumentValue<'value, bool> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        bool_fluent_value(*self.value)
    }
}

impl<'a, 'value, 'inner> IntoFluentArgumentValue<'a>
    for FluentBorrowedArgumentValue<'value, &'inner bool>
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        bool_fluent_value(**self.value)
    }
}

fn bool_fluent_value<'a>(value: bool) -> FluentValue<'a> {
    if value { "true" } else { "false" }.into()
}

impl<'a> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<&bool> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        match self.value {
            Some(value) => bool_fluent_value(*value),
            None => FluentValue::None,
        }
    }
}

impl<'a> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<&&bool> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        match self.value {
            Some(value) => bool_fluent_value(**value),
            None => FluentValue::None,
        }
    }
}

impl<'a, T> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<T>
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
        match self.value {
            Some(value) => value.to_fluent_string_with(localize).into(),
            None => FluentValue::None,
        }
    }
}

impl<'a, T> IntoFluentArgumentValue<'a> for &FluentOptionalArgumentValue<&T>
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
            Some(value) => (*value).clone().into_fluent_value(),
            None => FluentValue::None,
        }
    }
}

impl<'a, T> IntoFluentArgumentValue<'a> for FluentArgumentValue<Option<T>>
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
        match self.value {
            Some(value) => value.to_fluent_string_with(localize).into(),
            None => FluentValue::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, RwLock, mpsc};
    use std::time::Duration;

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

        let false_value = FluentArgumentValue::new(false).into_fluent_argument_value(&mut localize);
        assert_string(false_value, "false");

        let borrowed_bool = true;
        let borrowed_bool_value = FluentBorrowedArgumentValue::new(&borrowed_bool)
            .into_fluent_argument_value(&mut localize);
        assert_string(borrowed_bool_value, "true");
    }

    #[test]
    #[should_panic(expected = "ordinary arguments should not invoke nested localization")]
    fn panic_lookup_reports_unexpected_nested_localization() {
        let _ = panic_lookup("domain", "id", None);
    }

    #[test]
    fn argument_conversion_handles_optional_and_missing_values() {
        let mut localize = panic_lookup;
        let optional = Some("optional");
        let missing: Option<String> = None;
        let optional_number = Some(7i32);
        let optional_bool = Some(false);
        let missing_bool: Option<bool> = None;

        let optional_value = FluentOptionalArgumentValue::new(optional.as_ref())
            .into_fluent_argument_value(&mut localize);
        assert_string(optional_value, "optional");

        let missing_value = FluentOptionalArgumentValue::new(missing.as_ref())
            .into_fluent_argument_value(&mut localize);
        assert!(matches!(missing_value, FluentValue::None));

        let optional_number = FluentOptionalArgumentValue::new(optional_number.as_ref())
            .into_fluent_argument_value(&mut localize);
        assert_number(optional_number, 7.0);

        let optional_bool = FluentOptionalArgumentValue::new(optional_bool.as_ref())
            .into_fluent_argument_value(&mut localize);
        assert_string(optional_bool, "false");

        let missing_bool = FluentOptionalArgumentValue::new(missing_bool.as_ref())
            .into_fluent_argument_value(&mut localize);
        assert!(matches!(missing_bool, FluentValue::None));
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

    #[test]
    fn argument_conversion_localizes_optional_nested_messages_with_current_callback() {
        let mut localize =
            |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
                assert_eq!(domain, "nested-domain");
                assert_eq!(id, "nested-id");
                assert!(args.is_none());
                "optional nested value".to_string()
            };

        let value =
            FluentArgumentValue::new(Some(NestedMessage)).into_fluent_argument_value(&mut localize);
        assert_string(value, "optional nested value");

        let missing = FluentArgumentValue::new(Option::<NestedMessage>::None)
            .into_fluent_argument_value(&mut localize);
        assert!(matches!(missing, FluentValue::None));
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

    struct MissingMessage;

    impl FluentMessage for MissingMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            localize("missing-domain", "missing-id", None)
        }
    }

    struct CallbackOnlyMessage;

    impl FluentMessage for CallbackOnlyMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            localize("callback-domain", "callback-id", None)
        }
    }

    #[test]
    fn fluent_message_reference_impl_delegates_to_inner_message() {
        let message = NestedMessage;
        let message_ref = &message;
        let mut localize =
            |domain: &str, id: &str, _args: Option<&HashMap<&str, FluentValue<'_>>>| {
                format!("{domain}:{id}")
            };

        assert_eq!(
            FluentMessage::to_fluent_string_with(&message_ref, &mut localize),
            "nested-domain:nested-id"
        );
    }

    #[test]
    fn manual_fluent_message_uses_supplied_callback_for_lookup() {
        let mut called = false;
        let mut localize =
            |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
                called = true;
                assert_eq!(domain, "callback-domain");
                assert_eq!(id, "callback-id");
                assert!(args.is_none());
                "callback result".to_string()
            };

        assert_eq!(
            CallbackOnlyMessage.to_fluent_string_with(&mut localize),
            "callback result"
        );
        assert!(called);
    }

    #[test]
    fn fluent_localizer_reference_and_arc_impls_delegate_to_inner_localizer() {
        let localizer = StaticLocalizer { value: "Hello" };
        let localizer_ref = &localizer;
        let localizer_arc = Arc::new(StaticLocalizer { value: "Bonjour" });

        assert_eq!(localizer_ref.localize_message(&NestedMessage), "Hello");
        assert_eq!(localizer_arc.localize_message(&NestedMessage), "Bonjour");
        assert_eq!(
            FluentLocalizer::localize(&localizer_ref, "nested-id", None),
            Some("Hello".to_string())
        );
        assert_eq!(
            FluentLocalizer::localize_in_domain(&localizer_ref, "nested-domain", "nested-id", None,),
            Some("Hello".to_string())
        );
        assert_eq!(
            FluentLocalizer::localize_in_domain(&localizer_arc, "nested-domain", "nested-id", None,),
            Some("Bonjour".to_string())
        );
    }

    #[test]
    fn localizer_extension_localizes_typed_messages_with_id_fallback() {
        let localizer = StaticLocalizer { value: "Hello" };

        assert_eq!(
            FluentLocalizer::localize(&localizer, "nested-id", None),
            Some("Hello".to_string())
        );
        assert_eq!(
            FluentLocalizer::localize_in_domain(&localizer, "nested-domain", "nested-id", None),
            Some("Hello".to_string())
        );
        assert_eq!(localizer.localize_message(&MissingMessage), "missing-id");
    }

    #[test]
    fn localizer_extension_can_return_missing_typed_messages_without_id_fallback() {
        let localizer = StaticLocalizer { value: "Hello" };

        assert_eq!(
            localizer.try_localize_message(&NestedMessage),
            Some("Hello".to_string())
        );
        assert_eq!(localizer.try_localize_message(&MissingMessage), None);
    }

    struct MinimalScopedLocalizer;

    impl MinimalScopedLocalizer {
        fn lookup<'a>(
            &self,
            domain: &str,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            Some(format!("{domain}:{id}"))
        }
    }

    impl FluentLocalizer for MinimalScopedLocalizer {
        fn localize<'a>(
            &self,
            id: &str,
            args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            self.localize_in_domain(env!("CARGO_PKG_NAME"), id, args)
        }

        fn localize_in_domain<'a>(
            &self,
            domain: &str,
            id: &str,
            args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            self.lookup(domain, id, args)
        }

        fn with_lookup(
            &self,
            f: &mut dyn FnMut(
                &mut dyn for<'a> FnMut(
                    &str,
                    &str,
                    Option<&HashMap<&str, FluentValue<'a>>>,
                ) -> Option<String>,
            ),
        ) {
            let mut lookup =
                |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
                    self.localize_in_domain(domain, id, args)
                };
            f(&mut lookup);
        }
    }

    struct ScopedMessage;

    impl FluentMessage for ScopedMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            localize("custom-domain", "scoped-message", None)
        }
    }

    #[test]
    fn custom_localizer_with_lookup_invokes_callback_and_renders_typed_message() {
        assert_eq!(
            MinimalScopedLocalizer.localize_message(&ScopedMessage),
            "custom-domain:scoped-message"
        );
    }

    struct SkippingCallbackLocalizer;

    impl FluentLocalizer for SkippingCallbackLocalizer {
        fn localize<'a>(
            &self,
            _id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            None
        }

        fn localize_in_domain<'a>(
            &self,
            _domain: &str,
            _id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            None
        }

        fn with_lookup(
            &self,
            _f: &mut dyn FnMut(
                &mut dyn for<'a> FnMut(
                    &str,
                    &str,
                    Option<&HashMap<&str, FluentValue<'a>>>,
                ) -> Option<String>,
            ),
        ) {
        }
    }

    struct DoubleCallbackLocalizer;

    impl FluentLocalizer for DoubleCallbackLocalizer {
        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            Some(id.to_string())
        }

        fn localize_in_domain<'a>(
            &self,
            _domain: &str,
            id: &str,
            args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            self.localize(id, args)
        }

        fn with_lookup(
            &self,
            f: &mut dyn FnMut(
                &mut dyn for<'a> FnMut(
                    &str,
                    &str,
                    Option<&HashMap<&str, FluentValue<'a>>>,
                ) -> Option<String>,
            ),
        ) {
            let mut lookup =
                |_domain: &str, id: &str, _args: Option<&HashMap<&str, FluentValue<'_>>>| {
                    Some(id.to_string())
                };
            f(&mut lookup);
            f(&mut lookup);
        }
    }

    #[test]
    #[should_panic(expected = "FluentLocalizer::with_lookup must invoke its callback exactly once")]
    fn localize_message_panics_when_with_lookup_skips_callback() {
        SkippingCallbackLocalizer.localize_message(&NestedMessage);
    }

    #[test]
    #[should_panic(expected = "FluentLocalizer::with_lookup must invoke its callback exactly once")]
    fn try_localize_message_panics_when_with_lookup_invokes_callback_twice() {
        let _ = DoubleCallbackLocalizer.try_localize_message(&NestedMessage);
    }

    struct BlockingSwitchLocalizer {
        selected: RwLock<&'static str>,
        child_seen: Mutex<mpsc::Sender<()>>,
        continue_child: Mutex<mpsc::Receiver<()>>,
    }

    impl BlockingSwitchLocalizer {
        fn new(child_seen: mpsc::Sender<()>, continue_child: mpsc::Receiver<()>) -> Self {
            Self {
                selected: RwLock::new("en"),
                child_seen: Mutex::new(child_seen),
                continue_child: Mutex::new(continue_child),
            }
        }

        fn select(&self, language: &'static str) {
            *self
                .selected
                .write()
                .expect("test language lock should not be poisoned") = language;
        }

        fn selected(&self) -> &'static str {
            *self
                .selected
                .read()
                .expect("test language lock should not be poisoned")
        }

        fn render_lookup(&self, language: &'static str, domain: &str, id: &str) -> Option<String> {
            if domain != "switch-domain" {
                return None;
            }

            if id == "child" {
                self.child_seen
                    .lock()
                    .expect("test child sender lock should not be poisoned")
                    .send(())
                    .expect("test should receive child lookup notification");
                self.continue_child
                    .lock()
                    .expect("test child receiver lock should not be poisoned")
                    .recv()
                    .expect("test should release child lookup");
            }

            matches!(id, "child" | "parent").then(|| format!("{language}-{id}"))
        }
    }

    impl FluentLocalizer for BlockingSwitchLocalizer {
        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            let language = self.selected();
            self.render_lookup(language, "switch-domain", id)
        }

        fn localize_in_domain<'a>(
            &self,
            domain: &str,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            let language = self.selected();
            self.render_lookup(language, domain, id)
        }

        fn with_lookup(
            &self,
            f: &mut dyn FnMut(
                &mut dyn for<'a> FnMut(
                    &str,
                    &str,
                    Option<&HashMap<&str, FluentValue<'a>>>,
                ) -> Option<String>,
            ),
        ) {
            let selected = self
                .selected
                .read()
                .expect("test language lock should not be poisoned");
            let language = *selected;
            let mut lookup =
                |domain: &str, id: &str, _args: Option<&HashMap<&str, FluentValue<'_>>>| {
                    self.render_lookup(language, domain, id)
                };

            f(&mut lookup);
        }
    }

    struct BlockingParent;

    impl FluentMessage for BlockingParent {
        fn to_fluent_string_with(
            &self,
            localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            let child = localize("switch-domain", "child", None);
            let parent = localize("switch-domain", "parent", None);
            format!("{parent}:{child}")
        }
    }

    #[test]
    fn localize_message_keeps_one_lookup_scope_during_concurrent_language_switch() {
        let (child_seen_tx, child_seen_rx) = mpsc::channel();
        let (continue_child_tx, continue_child_rx) = mpsc::channel();
        let localizer = Arc::new(BlockingSwitchLocalizer::new(
            child_seen_tx,
            continue_child_rx,
        ));

        let render_localizer = Arc::clone(&localizer);
        let render = std::thread::spawn(move || render_localizer.localize_message(&BlockingParent));

        child_seen_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("render should reach the child lookup");

        let (switch_started_tx, switch_started_rx) = mpsc::channel();
        let (switch_done_tx, switch_done_rx) = mpsc::channel();
        let switch_localizer = Arc::clone(&localizer);
        let switch = std::thread::spawn(move || {
            switch_started_tx
                .send(())
                .expect("test should observe language switch start");
            switch_localizer.select("fr");
            switch_done_tx
                .send(())
                .expect("test should observe language switch completion");
        });

        switch_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("language switch thread should start");
        assert!(
            switch_done_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "language switch completed while typed message render was still in progress"
        );

        continue_child_tx
            .send(())
            .expect("test should release the child lookup");

        let rendered = render
            .join()
            .expect("render thread should complete without panicking");
        switch_done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("language switch should complete after render");
        switch
            .join()
            .expect("language switch thread should complete without panicking");

        assert_eq!(rendered, "en-parent:en-child");
        assert_eq!(localizer.selected(), "fr");
    }
}
