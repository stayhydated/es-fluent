use crate::{
    FluentValue,
    registry::{StaticFluentArgumentName, StaticFluentMessageKey},
};

/// Generated Fluent arguments keyed by validated static argument names.
///
/// The arguments are stored in the form Fluent formats with, so lookups pass
/// them through without conversion.
#[derive(Debug, Default)]
pub struct FluentArgs<'a> {
    values: es_fluent_manager_core::FluentArgumentMap<'a>,
}

impl<'a> FluentArgs<'a> {
    pub fn new() -> Self {
        Self {
            values: es_fluent_manager_core::FluentArgumentMap::default(),
        }
    }

    pub fn insert(&mut self, name: StaticFluentArgumentName, value: FluentValue<'a>) {
        self.values.set(name.as_str(), value);
    }

    pub fn as_raw(&self) -> &es_fluent_manager_core::FluentArgumentMap<'a> {
        &self.values
    }

    pub fn is_empty(&self) -> bool {
        self.values.iter().next().is_none()
    }
}

/// Render-time lookup callback used by [`super::FluentMessage`] implementations.
pub type FluentMessageLookup<'lookup> =
    dyn for<'a> FnMut(StaticFluentMessageKey, Option<&'a FluentArgs<'a>>) -> String + 'lookup;

/// Fallible render-time lookup callback supplied by [`super::FluentLocalizer`].
pub type FluentLocalizerLookup<'lookup> = dyn for<'a> FnMut(StaticFluentMessageKey, Option<&'a FluentArgs<'a>>) -> Option<String>
    + 'lookup;
