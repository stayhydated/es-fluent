use crate::{FluentMessage, FluentText};
use bevy::prelude::*;
use unic_langid::LanguageIdentifier;

/// A Bevy resource that holds the most recently requested `LanguageIdentifier`.
#[derive(Clone, Resource)]
pub struct RequestedLanguageId(pub LanguageIdentifier);

/// A Bevy resource that holds the currently published active `LanguageIdentifier`.
///
/// This is the locale that `LocaleChangedEvent` refers to and that
/// locale-aware UI should treat as active.
#[derive(Clone, Resource)]
pub struct ActiveLanguageId(pub LanguageIdentifier);

/// Requested locale plus the resolved locale used for bundle lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LanguageSelection {
    pub(crate) requested: LanguageIdentifier,
    pub(crate) resolved: LanguageIdentifier,
}

impl LanguageSelection {
    pub(crate) fn new(requested: LanguageIdentifier, resolved: LanguageIdentifier) -> Self {
        Self {
            requested,
            resolved,
        }
    }
}

/// Internal bookkeeping for a requested locale that is still waiting on a
/// ready accepted bundle chain.
#[derive(Clone, Default, Resource)]
pub(crate) struct PendingLanguageChange(pub(crate) Option<LanguageSelection>);

/// Returns the primary language subtag from a `LanguageIdentifier`.
///
/// For example, for `en-US`, this would return `en`.
pub fn primary_language(lang: &LanguageIdentifier) -> &str {
    lang.language.as_str()
}

/// A trait for types that can be constructed from a `LanguageIdentifier`.
///
/// This is useful for components that need to be initialized with locale-specific
/// data.
pub trait FromLocale {
    /// Creates an instance of `Self` from the given language identifier.
    fn from_locale(lang: &LanguageIdentifier) -> Self;
}

/// A trait for types that can be updated in place when the locale changes.
///
/// This allows preserving the state of a component while updating only the
/// locale-dependent fields.
pub trait RefreshForLocale {
    /// Refreshes the internal state of `self` based on the new language identifier.
    fn refresh_for_locale(&mut self, lang: &LanguageIdentifier);
}

/// Blanket implementation of `RefreshForLocale` for types that implement `FromLocale`.
///
/// This falls back to rebuilding the entire object if no specialized implementation
/// is provided.
impl<T> RefreshForLocale for T
where
    T: FromLocale,
{
    #[inline]
    fn refresh_for_locale(&mut self, lang: &LanguageIdentifier) {
        *self = T::from_locale(lang);
    }
}

/// A Bevy `Message` sent to request a change of the requested locale.
#[derive(Clone, Message)]
pub struct LocaleChangeEvent(pub LanguageIdentifier);

/// A Bevy `Message` sent after the active locale has been successfully published.
#[derive(Clone, Message)]
pub struct LocaleChangedEvent(pub LanguageIdentifier);

/// A Bevy system that listens for `LocaleChangedEvent`s and updates components
/// that implement `RefreshForLocale`.
pub fn update_values_on_locale_change<T>(
    mut locale_changed_events: MessageReader<LocaleChangedEvent>,
    mut query: Query<&mut FluentText<T>>,
) where
    T: RefreshForLocale + FluentMessage + Clone + Component,
{
    for event in locale_changed_events.read() {
        for mut fluent_text in query.iter_mut() {
            fluent_text.value.refresh_for_locale(&event.0);
        }
    }
}
