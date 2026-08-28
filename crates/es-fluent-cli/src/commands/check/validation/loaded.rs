use super::context::ValidationContext;
use crate::core::ValidationIssue;
use crate::ftl::LoadedFtlFile;
use es_fluent_shared::fluent::{FluentArgumentName, FluentDomain, FluentEntryId, FluentMessageKey};
use es_fluent_shared::resource::classify_fluent_entry;
use fluent_syntax::ast;
use indexmap::IndexMap;
use indexmap::map::Entry;
use std::collections::HashSet;

const SAME_AS_FALLBACK_MARKER: &str = "es-fluent: same-as-fallback";

#[derive(Clone)]
struct ActualKeyInfo {
    variables: HashSet<FluentArgumentName>,
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

pub(super) type FallbackKeys = IndexMap<FluentMessageKey, FallbackKeyInfo>;

pub(super) fn collect_fallback_keys(
    ctx: &ValidationContext<'_>,
    loaded_files: &[LoadedFtlFile],
) -> FallbackKeys {
    let mut fallback_keys = IndexMap::new();

    for file in loaded_files {
        let Some(domain) = domain_from_locale_relative_path(&file.relative_path) else {
            continue;
        };
        for entry in &file.resource.body {
            let ast::Entry::Message(msg) = entry else {
                continue;
            };
            let Ok(id) = FluentEntryId::try_new(msg.id.name.clone()) else {
                continue;
            };
            let key = FluentMessageKey::new(
                FluentDomain::try_new(ctx.owner.to_string()).expect("Cargo package owner is valid"),
                domain.clone(),
                id,
            );

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
                key.id().as_str(),
                locale,
                &expected_path,
                &expected_path,
            ));
            continue;
        };

        if actual.locale_relative_path != key_info.resource.locale_relative_path.as_str() {
            issues.push(ctx.missing_key_issue(
                key.id().as_str(),
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
                key.id().as_str(),
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
                key.id().as_str(),
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
                key.id().as_str(),
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
) -> IndexMap<FluentMessageKey, ActualKeyInfo> {
    let mut actual_keys: IndexMap<FluentMessageKey, ActualKeyInfo> = IndexMap::new();
    let mut seen_bundle_entries: IndexMap<(FluentDomain, FluentEntryId), String> = IndexMap::new();

    for file in loaded_files {
        let Some(domain) = domain_from_locale_relative_path(&file.relative_path) else {
            continue;
        };
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
                    let entry = match classify_fluent_entry(entry) {
                        Ok(Some(entry)) => entry,
                        Ok(None) => unreachable!("message entries are classified"),
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
                    let id = entry.id;
                    let resolves_message = entry.kind.resolves_message();
                    let key = FluentMessageKey::new(
                        FluentDomain::try_new(ctx.owner.to_string())
                            .expect("Cargo package owner is valid"),
                        domain.clone(),
                        id.clone(),
                    );
                    let bundle_key = (domain.clone(), id.clone());
                    if let Some(first_file) = seen_bundle_entries.get(&bundle_key) {
                        issues.push(ctx.duplicate_key_issue(
                            key.id().as_str(),
                            locale,
                            first_file,
                            &relative_path,
                            &header_link,
                        ));
                        allow_same_as_fallback = false;
                        continue;
                    }
                    seen_bundle_entries.insert(bundle_key, relative_path.clone());
                    if !resolves_message {
                        allow_same_as_fallback = false;
                        continue;
                    }

                    actual_keys.insert(
                        key,
                        ActualKeyInfo {
                            variables: collect_actual_variables(ctx, msg, locale, &file, issues),
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
                ast::Entry::Term(term) => {
                    let id = match classify_fluent_entry(entry) {
                        Ok(Some(entry)) => entry.id,
                        Ok(None) => unreachable!("term entries are classified"),
                        Err(error) => {
                            issues.push(ctx.syntax_error_issue(
                                locale,
                                &file.abs_path,
                                format!("Invalid FTL term id '-{}': {error}", term.id.name),
                            ));
                            allow_same_as_fallback = false;
                            continue;
                        },
                    };
                    let bundle_key = (domain.clone(), id.clone());
                    if let Some(first_file) = seen_bundle_entries.get(&bundle_key) {
                        issues.push(ctx.duplicate_key_issue(
                            id.as_str(),
                            locale,
                            first_file,
                            &relative_path,
                            &header_link,
                        ));
                    } else {
                        seen_bundle_entries.insert(bundle_key, relative_path.clone());
                    }
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

fn domain_from_locale_relative_path(path: &std::path::Path) -> Option<FluentDomain> {
    let first = path.components().next()?.as_os_str().to_str()?;
    let domain = first.strip_suffix(".ftl").unwrap_or(first);
    FluentDomain::try_new(domain.to_string()).ok()
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
