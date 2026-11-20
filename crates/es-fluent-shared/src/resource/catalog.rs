use std::collections::{BTreeMap, HashMap, HashSet};

use crate::fluent::{FluentDomain, FluentEntryId, FluentIdentifierError};

/// File written to `OUT_DIR` by `es-fluent-build` with the i18n inputs
/// es-fluent macros read: `i18n.toml`, discovered locales and resources, and
/// the fallback-message catalog.
pub const BUILD_MANIFEST_FILE_NAME: &str = "es-fluent-build-manifest.json";

/// Rust compilation environment variable containing the build manifest path.
#[doc(hidden)]
pub const BUILD_MANIFEST_ENV: &str = "ES_FLUENT_BUILD_MANIFEST";

/// Rust compilation environment variable containing the content hash of the
/// build manifest.
///
/// Macros reference it through `env!` so rustc records the hash in dep-info.
/// Compiler caches that key on dep-info, such as sccache or kache, then
/// re-expand macros exactly when the manifest they read changes.
#[doc(hidden)]
pub const BUILD_MANIFEST_HASH_ENV: &str = "ES_FLUENT_BUILD_MANIFEST_HASH";

/// Internal environment marker for CLI inventory builds that defer catalog validation.
#[doc(hidden)]
pub const INVENTORY_RUNNER_ENV: &str = "ES_FLUENT_INVENTORY_RUNNER";

/// The kind of a top-level Fluent entry relevant to runtime message lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluentCatalogEntryKind {
    /// A message with a value that can be resolved at runtime.
    Message,
    /// A message containing attributes but no resolvable value.
    MessageWithoutValue,
    /// A Fluent term.
    Term,
}

impl FluentCatalogEntryKind {
    /// Returns whether this entry can satisfy a typed message lookup.
    pub const fn resolves_message(self) -> bool {
        matches!(self, Self::Message)
    }
}

/// A typed top-level entry extracted from a parsed Fluent resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FluentCatalogEntry {
    /// The shared message/term identifier.
    pub id: FluentEntryId,
    /// The entry kind.
    pub kind: FluentCatalogEntryKind,
}

/// Classifies one parsed Fluent entry for shared build and CLI validation.
pub fn classify_fluent_entry(
    entry: &fluent_syntax::ast::Entry<String>,
) -> Result<Option<FluentCatalogEntry>, FluentIdentifierError> {
    let (name, kind) = match entry {
        fluent_syntax::ast::Entry::Message(message) => (
            message.id.name.as_str(),
            if message.value.is_some() {
                FluentCatalogEntryKind::Message
            } else {
                FluentCatalogEntryKind::MessageWithoutValue
            },
        ),
        fluent_syntax::ast::Entry::Term(term) => {
            (term.id.name.as_str(), FluentCatalogEntryKind::Term)
        },
        _ => return Ok(None),
    };

    FluentEntryId::try_new(name.to_string()).map(|id| Some(FluentCatalogEntry { id, kind }))
}

/// Errors produced while building a fallback-locale message catalog.
#[derive(Debug, thiserror::Error)]
pub enum FallbackCatalogError {
    /// The Fluent source could not be parsed.
    #[error("invalid Fluent syntax: {details}")]
    Syntax { details: String },
    /// A top-level entry identifier is invalid.
    #[error("invalid Fluent entry identifier: {0}")]
    Identifier(#[from] FluentIdentifierError),
    /// A message or term ID appears more than once in one domain.
    #[error("duplicate Fluent entry '{id}' in domain '{domain}'")]
    Duplicate { domain: String, id: String },
}

/// Resolvable fallback messages collected across package-local domains.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FallbackCatalog {
    entries: BTreeMap<(FluentDomain, FluentEntryId), FluentCatalogEntryKind>,
}

impl FallbackCatalog {
    /// Parses and adds one domain resource to the catalog.
    pub fn insert_source(
        &mut self,
        domain: &FluentDomain,
        source: String,
    ) -> Result<(), FallbackCatalogError> {
        let resource = fluent_syntax::parser::parse(source).map_err(|(_, errors)| {
            FallbackCatalogError::Syntax {
                details: format!("{errors:?}"),
            }
        })?;
        self.insert_resource(domain, &resource)
    }

    /// Adds one parsed domain resource to the catalog.
    pub fn insert_resource(
        &mut self,
        domain: &FluentDomain,
        resource: &fluent_syntax::ast::Resource<String>,
    ) -> Result<(), FallbackCatalogError> {
        for entry in &resource.body {
            let Some(entry) = classify_fluent_entry(entry)? else {
                continue;
            };
            let key = (domain.clone(), entry.id.clone());
            if self.entries.insert(key, entry.kind).is_some() {
                return Err(FallbackCatalogError::Duplicate {
                    domain: domain.as_str().to_string(),
                    id: entry.id.as_str().to_string(),
                });
            }
        }
        Ok(())
    }

    /// Returns the messages that resolve a typed lookup, ordered by domain and ID.
    pub fn resolvable_messages(&self) -> impl Iterator<Item = (&FluentDomain, &FluentEntryId)> {
        self.entries
            .iter()
            .filter(|(_, kind)| kind.resolves_message())
            .map(|((domain, id), _)| (domain, id))
    }
}

/// Resolvable fallback messages indexed for repeated lookups.
///
/// Derives check every generated message against the catalog, so it is
/// indexed once per package build instead of scanned for each message.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FallbackCatalogIndex {
    messages: HashMap<Box<str>, HashSet<Box<str>>>,
}

impl FallbackCatalogIndex {
    /// Returns whether the catalog resolves `id` in `domain`.
    pub fn contains(&self, domain: &str, id: &str) -> bool {
        self.messages
            .get(domain)
            .is_some_and(|ids| ids.contains(id))
    }
}

impl<D: Into<Box<str>>, I: Into<Box<str>>> FromIterator<(D, I)> for FallbackCatalogIndex {
    fn from_iter<T: IntoIterator<Item = (D, I)>>(messages: T) -> Self {
        let mut index = HashMap::<Box<str>, HashSet<Box<str>>>::new();
        for (domain, id) in messages {
            index.entry(domain.into()).or_default().insert(id.into());
        }
        Self { messages: index }
    }
}
