use crate::{FluentValue, registry::StaticFluentVariantKey};

use super::{FluentMessage, FluentMessageLookup};

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

    #[cfg(feature = "icu-datetime")]
    pub(crate) fn into_inner(self) -> T {
        self.value
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

    #[cfg(feature = "icu-datetime")]
    pub(crate) fn into_inner(self) -> &'a T {
        self.value
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

    #[cfg(feature = "icu-datetime")]
    pub(crate) fn into_inner(self) -> Option<T> {
        self.value
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
    fn into_fluent_argument_value(self, localize: &mut FluentMessageLookup<'_>) -> FluentValue<'a>;
}

impl<'a, T> IntoFluentArgumentValue<'a> for FluentArgumentValue<T>
where
    T: FluentMessage,
{
    fn into_fluent_argument_value(self, localize: &mut FluentMessageLookup<'_>) -> FluentValue<'a> {
        self.value.to_fluent_string_with(localize).into()
    }
}

impl<'a, 'value, T> IntoFluentArgumentValue<'a> for FluentBorrowedArgumentValue<'value, T>
where
    T: FluentMessage + ?Sized,
{
    fn into_fluent_argument_value(self, localize: &mut FluentMessageLookup<'_>) -> FluentValue<'a> {
        self.value.to_fluent_string_with(localize).into()
    }
}

impl<'a, T> IntoFluentArgumentValue<'a> for &FluentArgumentValue<T>
where
    T: Clone + IntoFluentValue<'a>,
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
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
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        (*self.value).clone().into_fluent_value()
    }
}

impl<'a> IntoFluentArgumentValue<'a> for FluentArgumentValue<bool> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        bool_fluent_value(self.value)
    }
}

impl<'a, 'value> IntoFluentArgumentValue<'a> for FluentBorrowedArgumentValue<'value, bool> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        bool_fluent_value(*self.value)
    }
}

impl<'a, 'value, 'inner> IntoFluentArgumentValue<'a>
    for FluentBorrowedArgumentValue<'value, &'inner bool>
{
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        bool_fluent_value(**self.value)
    }
}

impl<'a> IntoFluentArgumentValue<'a> for FluentArgumentValue<StaticFluentVariantKey> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        self.value.as_str().into()
    }
}

impl<'a> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<StaticFluentVariantKey> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
    ) -> FluentValue<'a> {
        match self.value {
            Some(value) => value.as_str().into(),
            None => FluentValue::None,
        }
    }
}

fn bool_fluent_value<'a>(value: bool) -> FluentValue<'a> {
    if value { "true" } else { "false" }.into()
}

impl<'a> IntoFluentArgumentValue<'a> for FluentOptionalArgumentValue<&bool> {
    fn into_fluent_argument_value(
        self,
        _localize: &mut FluentMessageLookup<'_>,
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
        _localize: &mut FluentMessageLookup<'_>,
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
    fn into_fluent_argument_value(self, localize: &mut FluentMessageLookup<'_>) -> FluentValue<'a> {
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
        _localize: &mut FluentMessageLookup<'_>,
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
    fn into_fluent_argument_value(self, localize: &mut FluentMessageLookup<'_>) -> FluentValue<'a> {
        match self.value {
            Some(value) => value.to_fluent_string_with(localize).into(),
            None => FluentValue::None,
        }
    }
}
