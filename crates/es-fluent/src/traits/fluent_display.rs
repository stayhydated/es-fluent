use std::fmt;

pub trait FluentDisplay {
    fn fluent_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub trait ToFluentString {
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
