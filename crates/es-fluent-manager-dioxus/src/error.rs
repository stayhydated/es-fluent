#[derive(Clone, Debug)]
pub enum DioxusAssetI18nContextError {
    MissingContext,
}

impl std::fmt::Display for DioxusAssetI18nContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingContext => f.write_str("missing Dioxus asset i18n provider"),
        }
    }
}

impl std::error::Error for DioxusAssetI18nContextError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn dioxus_asset_i18n_context_error_reports_missing_context() {
        let missing = DioxusAssetI18nContextError::MissingContext;

        assert_eq!(missing.to_string(), "missing Dioxus asset i18n provider");
        assert!(missing.source().is_none());
    }
}
