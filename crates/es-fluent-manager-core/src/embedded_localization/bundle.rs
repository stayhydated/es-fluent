use fluent_bundle::FluentError;
use unic_langid::LanguageIdentifier;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleBuildError {
    module_name: String,
    language: LanguageIdentifier,
    diagnostics: Vec<String>,
}

impl BundleBuildError {
    pub(super) fn from_add_errors(
        module_name: &str,
        language: &LanguageIdentifier,
        add_errors: Vec<Vec<FluentError>>,
    ) -> Self {
        let diagnostics = add_errors
            .into_iter()
            .enumerate()
            .map(|(resource_index, errors)| {
                let messages = errors
                    .into_iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("resource #{resource_index}: {messages}")
            })
            .collect();

        Self {
            module_name: module_name.to_string(),
            language: language.clone(),
            diagnostics,
        }
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    pub fn language(&self) -> &LanguageIdentifier {
        &self.language
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

impl std::fmt::Display for BundleBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to build a Fluent bundle for module '{}' and language '{}': {}",
            self.module_name,
            self.language,
            self.diagnostics.join(" | ")
        )
    }
}

impl std::error::Error for BundleBuildError {}
