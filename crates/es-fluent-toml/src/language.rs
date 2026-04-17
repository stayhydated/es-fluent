use crate::I18nConfigError;
use es_fluent_shared::{CanonicalLanguageIdentifierError, parse_canonical_language_identifier};
use fs_err as fs;
use std::io;
use unic_langid::LanguageIdentifier;

#[derive(Debug)]
pub(crate) struct ParsedLanguageEntry {
    pub(crate) raw_name: String,
    pub(crate) language: LanguageIdentifier,
}

/// Parse a directory entry as a language identifier.
///
/// Returns `Ok(None)` if the entry is not a directory.
pub(crate) fn parse_language_entry(
    entry: fs::DirEntry,
) -> Result<Option<ParsedLanguageEntry>, I18nConfigError> {
    if !entry
        .file_type()
        .map_err(I18nConfigError::ReadError)?
        .is_dir()
    {
        return Ok(None);
    }

    let raw_name = entry.file_name();
    let name = raw_name.into_string().map_err(|raw| {
        I18nConfigError::ReadError(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Assets directory contains a non UTF-8 entry: {:?}", raw),
        ))
    })?;

    let lang = parse_canonical_language_identifier(&name).map_err(|err| match err {
        CanonicalLanguageIdentifierError::Invalid { source, .. } => {
            I18nConfigError::InvalidLanguageIdentifier {
                name: name.clone(),
                source,
            }
        },
        CanonicalLanguageIdentifierError::NonCanonical { canonical, .. } => {
            I18nConfigError::NonCanonicalLanguageIdentifier {
                name: name.clone(),
                canonical,
            }
        },
    })?;

    Ok(Some(ParsedLanguageEntry {
        raw_name: name,
        language: lang,
    }))
}
