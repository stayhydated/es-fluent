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
    /// Attempts to render a derived typed message through this explicit
    /// localizer.
    ///
    /// Returns `None` if any lookup in the message tree is missing. Use
    /// `localize_message(...)` when a display fallback to the message ID is
    /// desired instead.
    fn try_localize_message<T>(&self, message: &T) -> Option<String>
    where
        T: FluentMessage + ?Sized,
    {
        let mut missing = false;
        let value = message.to_fluent_string_with(&mut |domain, id, args| {
            self.localize_in_domain(domain, id, args)
                .unwrap_or_else(|| {
                    missing = true;
                    String::new()
                })
        });

        if missing { None } else { Some(value) }
    }

    /// Renders a derived typed message through this explicit localizer.
    fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        message.to_fluent_string_with(&mut |domain, id, args| {
            self.localize_in_domain(domain, id, args)
                .unwrap_or_else(|| {
                    tracing::warn!(domain, message_id = id, "missing Fluent message");
                    id.to_string()
                })
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
mod tests;
