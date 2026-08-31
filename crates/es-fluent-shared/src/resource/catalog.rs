use std::collections::BTreeMap;

use crate::fluent::{FluentDomain, FluentEntryId, FluentIdentifierError};

/// File emitted by `es-fluent-build` for strict fallback-locale validation.
pub const FALLBACK_CATALOG_FILE_NAME: &str = "es-fluent-fallback.catalog";

/// Rust compilation environment variable containing the generated catalog path.
#[doc(hidden)]
pub const FALLBACK_CATALOG_ENV: &str = "ES_FLUENT_FALLBACK_CATALOG";

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

    /// Encodes resolvable messages for const lookup from generated derive code.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        for ((domain, id), kind) in &self.entries {
            if !kind.resolves_message() {
                continue;
            }
            encoded.extend_from_slice(domain.as_str().as_bytes());
            encoded.push(b'\t');
            encoded.extend_from_slice(id.as_str().as_bytes());
            encoded.push(b'\n');
        }
        encoded
    }
}

/// Checks an encoded fallback catalog from generated const validation code.
pub const fn fallback_catalog_contains(catalog: &[u8], domain: &str, id: &str) -> bool {
    let domain = domain.as_bytes();
    let id = id.as_bytes();
    let mut offset = 0;

    while offset < catalog.len() {
        let mut cursor = offset;
        let mut domain_index = 0;
        while domain_index < domain.len()
            && cursor < catalog.len()
            && catalog[cursor] == domain[domain_index]
        {
            cursor += 1;
            domain_index += 1;
        }
        if domain_index == domain.len() && cursor < catalog.len() && catalog[cursor] == b'\t' {
            cursor += 1;
            let mut id_index = 0;
            while id_index < id.len() && cursor < catalog.len() && catalog[cursor] == id[id_index] {
                cursor += 1;
                id_index += 1;
            }
            if id_index == id.len() && (cursor == catalog.len() || catalog[cursor] == b'\n') {
                return true;
            }
        }

        while offset < catalog.len() && catalog[offset] != b'\n' {
            offset += 1;
        }
        offset += 1;
    }

    false
}
