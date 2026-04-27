use crate::FluentValue;
use std::collections::HashMap;
use std::fmt;

/// This trait is similar to `std::fmt::Display`, but it is used for formatting
/// types that can be displayed in a Fluent message.
pub trait FluentDisplay {
    /// Formats the value using the given formatter.
    fn fluent_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<T: FluentDisplay + ?Sized> FluentDisplay for &T {
    fn fluent_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fluent_fmt(f)
    }
}

/// This trait is automatically implemented for any type that implements
/// `FluentDisplay`.
pub trait ToFluentString {
    /// Converts the type into a Fluent (i18n translated) string.
    fn to_fluent_string(&self) -> String;
}

/// A typed Fluent message that can be resolved by an explicit localization
/// backend instead of the process-global `es-fluent` context.
///
/// Derive macros implement this trait for `#[derive(EsFluent)]` and generated
/// variant enums. Runtime managers use it to keep typed message call sites while
/// routing lookup through a request or component-scoped manager.
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

/// Wrapper used by generated skipped-variant `FluentMessage` implementations
/// to prefer context-bound formatting when possible while preserving the
/// historic `ToFluentString` fallback for skipped wrapper variants.
#[doc(hidden)]
pub struct FluentMessageStringValue<T> {
    value: T,
}

impl<T> FluentMessageStringValue<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

#[doc(hidden)]
pub trait IntoFluentMessageString {
    fn into_fluent_message_string(
        self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String;
}

impl<T> IntoFluentMessageString for FluentMessageStringValue<T>
where
    T: FluentMessage,
{
    fn into_fluent_message_string(
        self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        self.value.to_fluent_string_with(localize)
    }
}

impl<T> IntoFluentMessageString for &FluentMessageStringValue<T>
where
    T: ToFluentString,
{
    fn into_fluent_message_string(
        self,
        _localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        self.value.to_fluent_string()
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
    T: Clone + Into<FluentValue<'a>>,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut dyn for<'b> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'b>>>,
        ) -> String,
    ) -> FluentValue<'a> {
        self.value.clone().into()
    }
}

impl<T: FluentDisplay + ?Sized> ToFluentString for T {
    fn to_fluent_string(&self) -> String {
        struct FluentDisplayWrapper<'a, T: ?Sized> {
            inner: &'a T,
        }

        impl<'a, T: FluentDisplay + ?Sized> fmt::Display for FluentDisplayWrapper<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.inner.fluent_fmt(f)
            }
        }

        FluentDisplayWrapper { inner: self }.to_string()
    }
}
