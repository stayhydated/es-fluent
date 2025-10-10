use std::fmt;

/// A trait for formatting a type as a Fluent message.
///
/// This trait is similar to `std::fmt::Display`, but it is used for formatting
/// types that can be displayed in a Fluent message.
pub trait FluentDisplay {
    /// Formats the value using the given formatter.
    fn fluent_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// A trait for converting a type into a Fluent string.
///
/// This trait is automatically implemented for any type that implements
/// `FluentDisplay`.
pub trait ToFluentString {
    /// Converts the type into a Fluent string.
    fn to_fluent_string(&self) -> String;
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
