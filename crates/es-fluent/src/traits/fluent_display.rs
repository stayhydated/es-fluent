use std::fmt;

/// A trait for types that can be displayed in a Fluent message.
///
/// This is a counterpart to `std::fmt::Display`, but specifically for generating
/// strings that are meant to be used with Fluent. The derive macro for `es-fluent`
/// can implement this trait for your types.
pub trait FluentDisplay {
    /// Formats the value using the given formatter.
    fn fluent_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// Extension trait that provides `to_fluent_string`.
pub trait ToFluentString {
    /// Converts a value to a `String` for use in Fluent.
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
