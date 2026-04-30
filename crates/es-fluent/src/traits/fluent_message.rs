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
mod tests;
