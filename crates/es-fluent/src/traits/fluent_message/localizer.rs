use crate::registry::StaticFluentMessageKey;
use es_fluent_manager_core::FluentManager;
use std::sync::Arc;

use super::{FluentArgs, FluentLocalizerLookup, FluentMessageLookup};

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
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String;
}

#[diagnostic::do_not_recommend]
impl<T: FluentMessage + ?Sized> FluentMessage for &T {
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String {
        (**self).to_fluent_string_with(localize)
    }
}

/// Runtime context that resolves Fluent message IDs for typed message values.
///
/// Managers and framework adapters implement this trait so callers can pass
/// the active localization context explicitly.
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
    /// Localizes a fully scoped static message key.
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&'a FluentArgs<'a>>,
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
    /// # use es_fluent::{FluentArgs, FluentLocalizer};
    /// # use es_fluent::registry::StaticFluentMessageKey;
    /// struct MyLocalizer;
    ///
    /// fn lookup(
    ///     key: StaticFluentMessageKey,
    ///     _args: Option<&FluentArgs<'_>>,
    /// ) -> Option<String> {
    ///     Some(format!("{}:{}:{}", key.owner(), key.domain(), key.id()))
    /// }
    ///
    /// impl FluentLocalizer for MyLocalizer {
    ///     fn localize<'a>(
    ///         &self,
    ///         key: StaticFluentMessageKey,
    ///         args: Option<&FluentArgs<'a>>,
    ///     ) -> Option<String> {
    ///         lookup(key, args)
    ///     }
    ///
    ///     fn with_lookup(
    ///         &self,
    ///         f: &mut dyn FnMut(&mut es_fluent::FluentLocalizerLookup<'_>),
    ///     ) {
    ///         let mut lookup = lookup;
    ///         f(&mut lookup);
    ///     }
    /// }
    /// ```
    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        let mut lookup =
            |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| self.localize(key, args);
        f(&mut lookup);
    }
}

impl FluentLocalizer for FluentManager {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        FluentManager::localize(self, key, args.map(FluentArgs::as_raw))
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        FluentManager::with_lookup(self, &mut |lookup| {
            let mut typed_lookup = |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| {
                lookup(key, args.map(FluentArgs::as_raw))
            };
            f(&mut typed_lookup);
        });
    }
}

impl<T: FluentLocalizer + ?Sized> FluentLocalizer for &T {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        (**self).localize(key, args)
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        (**self).with_lookup(f);
    }
}

impl<T: FluentLocalizer + ?Sized> FluentLocalizer for Arc<T> {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        (**self).localize(key, args)
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
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
    /// Returns `None` if any lookup in the message tree is missing. Use this
    /// method when missing resources are an expected condition that the caller
    /// handles explicitly.
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

            value = Some(message.to_fluent_string_with(&mut |key, args| {
                lookup(key, args).unwrap_or_else(|| {
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
    ///
    /// A key generated under the package-local `fallback-str` policy returns
    /// its snake_case Rust source name. Strict keys still panic.
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

            value = Some(message.to_fluent_string_with(&mut |key, args| {
                lookup(key, args)
                    .unwrap_or_else(|| super::super::missing_fluent_value(key, "message"))
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
