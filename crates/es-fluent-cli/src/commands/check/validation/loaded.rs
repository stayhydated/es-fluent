use super::context::ValidationContext;
use crate::core::ValidationIssue;
use crate::ftl::LoadedFtlFile;
use es_fluent_shared::fluent::{FluentArgumentName, FluentEntryId};
use fluent_syntax::ast;
use indexmap::IndexMap;
use indexmap::map::Entry;
use std::collections::HashSet;

const SAME_AS_FALLBACK_MARKER: &str = "es-fluent: same-as-fallback";

#[derive(Clone)]
struct ActualKeyInfo {
    variables: HashSet<FluentArgumentName>,
    file_path: String,
    locale_relative_path: String,
    header_link: String,
    translation_fingerprint: String,
    allow_same_as_fallback: bool,
}

#[derive(Clone)]
pub(super) struct FallbackKeyInfo {
    locale_relative_path: String,
    translation_fingerprint: String,
}

pub(super) type FallbackKeys = IndexMap<FluentEntryId, FallbackKeyInfo>;

pub(super) fn collect_fallback_keys(loaded_files: &[LoadedFtlFile]) -> FallbackKeys {
    let mut fallback_keys = IndexMap::new();

    for file in loaded_files {
        for entry in &file.resource.body {
            let ast::Entry::Message(msg) = entry else {
                continue;
            };
            let Ok(key) = FluentEntryId::try_new(msg.id.name.clone()) else {
                continue;
            };

            if let Entry::Vacant(slot) = fallback_keys.entry(key) {
                slot.insert(FallbackKeyInfo {
                    locale_relative_path: crate::utils::paths::slash_path(&file.relative_path),
                    translation_fingerprint: message_translation_fingerprint(msg),
                });
            }
        }
    }

    fallback_keys
}

pub(super) fn validate_loaded_ftl_files(
    ctx: &ValidationContext<'_>,
    loaded_files: Vec<LoadedFtlFile>,
    locale: &str,
    fallback_locale: &str,
    fallback_keys: Option<&FallbackKeys>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let actual_keys = collect_actual_keys(ctx, loaded_files, locale, &mut issues);

    for (key, key_info) in ctx.expected_keys {
        let expected_path = ctx.expected_resource_path(locale, key_info);
        let Some(actual) = actual_keys.get(key) else {
            issues.push(ctx.missing_key_issue(
                key.as_str(),
                locale,
                &expected_path,
                &expected_path,
            ));
            continue;
        };

        if actual.locale_relative_path != key_info.resource.locale_relative_path.as_str() {
            issues.push(ctx.missing_key_issue(
                key.as_str(),
                locale,
                &expected_path,
                &actual.header_link,
            ));
            continue;
        };

        if let Some(fallback_keys) = fallback_keys
            && locale != fallback_locale
            && let Some(fallback) = fallback_keys.get(key)
            && fallback.locale_relative_path == key_info.resource.locale_relative_path.as_str()
            && fallback.translation_fingerprint == actual.translation_fingerprint
            && !actual.allow_same_as_fallback
        {
            issues.push(ctx.untranslated_message_issue(
                key.as_str(),
                locale,
                fallback_locale,
                &actual.header_link,
            ));
        }

        for variable in &key_info.variables {
            if actual.variables.contains(variable) {
                continue;
            }

            issues.push(ctx.missing_variable_issue(
                key.as_str(),
                variable.as_str(),
                locale,
                &actual.header_link,
                key_info.source_file.as_ref().map(|file| file.as_str()),
                key_info.source_line.map(|line| line.get()),
            ));
        }

        for variable in &actual.variables {
            if key_info.variables.contains(variable) {
                continue;
            }

            issues.push(ctx.unexpected_variable_issue(
                key.as_str(),
                variable.as_str(),
                locale,
                &actual.header_link,
            ));
        }
    }

    issues
}

fn collect_actual_keys(
    ctx: &ValidationContext<'_>,
    loaded_files: Vec<LoadedFtlFile>,
    locale: &str,
    issues: &mut Vec<ValidationIssue>,
) -> IndexMap<FluentEntryId, ActualKeyInfo> {
    let mut actual_keys: IndexMap<FluentEntryId, ActualKeyInfo> = IndexMap::new();

    for file in loaded_files {
        let relative_path = ctx.to_relative_path(&file.abs_path);
        let header_link = ctx.format_terminal_link(
            &relative_path,
            &format!("file://{}", file.abs_path.display()),
        );

        let mut allow_same_as_fallback = false;
        for entry in &file.resource.body {
            match entry {
                ast::Entry::Comment(comment) => {
                    allow_same_as_fallback |= comment
                        .content
                        .iter()
                        .any(|line| line.contains(SAME_AS_FALLBACK_MARKER));
                },
                ast::Entry::Message(msg) => {
                    let key = match FluentEntryId::try_new(msg.id.name.clone()) {
                        Ok(key) => key,
                        Err(error) => {
                            issues.push(ctx.syntax_error_issue(
                                locale,
                                &file.abs_path,
                                format!("Invalid FTL message id '{}': {error}", msg.id.name),
                            ));
                            allow_same_as_fallback = false;
                            continue;
                        },
                    };
                    if let Some(previous) = actual_keys.get(&key) {
                        issues.push(ctx.duplicate_key_issue(
                            key.as_str(),
                            locale,
                            &previous.file_path,
                            &relative_path,
                            &header_link,
                        ));
                        allow_same_as_fallback = false;
                        continue;
                    }

                    actual_keys.insert(
                        key,
                        ActualKeyInfo {
                            variables: collect_actual_variables(ctx, msg, locale, &file, issues),
                            file_path: relative_path.clone(),
                            locale_relative_path: crate::utils::paths::slash_path(
                                &file.relative_path,
                            ),
                            header_link: header_link.clone(),
                            translation_fingerprint: message_translation_fingerprint(msg),
                            allow_same_as_fallback,
                        },
                    );
                    allow_same_as_fallback = false;
                },
                _ => {
                    allow_same_as_fallback = false;
                },
            }
        }
    }

    actual_keys
}

fn collect_actual_variables(
    ctx: &ValidationContext<'_>,
    msg: &ast::Message<String>,
    locale: &str,
    file: &LoadedFtlFile,
    issues: &mut Vec<ValidationIssue>,
) -> HashSet<FluentArgumentName> {
    crate::ftl::extract_variables_from_message(msg)
        .into_iter()
        .filter_map(
            |variable| match FluentArgumentName::try_new(variable.clone()) {
                Ok(variable) => Some(variable),
                Err(error) => {
                    issues.push(ctx.syntax_error_issue(
                        locale,
                        &file.abs_path,
                        format!(
                            "Invalid FTL variable '${}' in message '{}': {error}",
                            variable, msg.id.name
                        ),
                    ));
                    None
                },
            },
        )
        .collect()
}

fn message_translation_fingerprint(msg: &ast::Message<String>) -> String {
    let resource = ast::Resource {
        body: vec![ast::Entry::Message(msg.clone())],
    };
    fluent_syntax::serializer::serialize(&resource)
}
