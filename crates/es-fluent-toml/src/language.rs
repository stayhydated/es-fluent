use crate::I18nConfigError;
use es_fluent_shared::CanonicalLanguageIdentifierError;
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

    let lang =
        es_fluent_shared::parse_canonical_language_identifier(&name).map_err(|err| match err {
            CanonicalLanguageIdentifierError::Invalid { source, .. } => {
                I18nConfigError::InvalidLanguageIdentifier {
                    name: name.clone(),
                    source,
                }
            },
            CanonicalLanguageIdentifierError::IcuInvalid { details, .. } => {
                I18nConfigError::IcuLanguageIdentifier {
                    name: name.clone(),
                    details,
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

#[cfg(test)]
mod tests {
    use super::*;
    use fs_err as fs;

    fn first_entry(path: &std::path::Path) -> fs::DirEntry {
        fs::read_dir(path)
            .expect("read dir")
            .next()
            .expect("expected one entry")
            .expect("dir entry")
    }

    #[test]
    fn parse_language_entry_returns_none_for_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.txt"), "ignored").expect("write file");

        let parsed = parse_language_entry(first_entry(temp.path())).expect("parse entry");
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_language_entry_returns_canonical_language_for_directories() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("en-US")).expect("create locale dir");

        let parsed = parse_language_entry(first_entry(temp.path()))
            .expect("parse entry")
            .expect("directory entry should be parsed");
        assert_eq!(parsed.raw_name, "en-US");
        assert_eq!(parsed.language.to_string(), "en-US");
    }

    #[test]
    fn parse_language_entry_rejects_invalid_language_directories() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("not_a_language")).expect("create locale dir");

        let error = parse_language_entry(first_entry(temp.path())).unwrap_err();
        assert!(matches!(
            error,
            I18nConfigError::InvalidLanguageIdentifier { .. }
        ));
    }

    #[test]
    fn parse_language_entry_rejects_non_canonical_language_directories() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("en-us")).expect("create locale dir");

        let error = parse_language_entry(first_entry(temp.path())).unwrap_err();
        assert!(matches!(
            error,
            I18nConfigError::NonCanonicalLanguageIdentifier { .. }
        ));
    }
}
