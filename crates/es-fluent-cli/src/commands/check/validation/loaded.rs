use super::context::ValidationContext;
use crate::core::ValidationIssue;
use crate::ftl::LoadedFtlFile;
use es_fluent_shared::fluent::{FluentArgumentName, FluentEntryId};
use fluent_syntax::ast;
use indexmap::IndexMap;
use std::collections::HashSet;

#[derive(Clone)]
struct ActualKeyInfo {
    variables: HashSet<FluentArgumentName>,
    file_path: String,
    locale_relative_path: String,
    header_link: String,
}

pub(super) fn validate_loaded_ftl_files(
    ctx: &ValidationContext<'_>,
    loaded_files: Vec<LoadedFtlFile>,
    locale: &str,
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

        for entry in &file.resource.body {
            if let ast::Entry::Message(msg) = entry {
                let key = match FluentEntryId::try_new(msg.id.name.clone()) {
                    Ok(key) => key,
                    Err(error) => {
                        issues.push(ctx.syntax_error_issue(
                            locale,
                            &file.abs_path,
                            format!("Invalid FTL message id '{}': {error}", msg.id.name),
                        ));
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
                    continue;
                }

                actual_keys.insert(
                    key,
                    ActualKeyInfo {
                        variables: collect_actual_variables(ctx, msg, locale, &file, issues),
                        file_path: relative_path.clone(),
                        locale_relative_path: file
                            .relative_path
                            .to_string_lossy()
                            .replace('\\', "/"),
                        header_link: header_link.clone(),
                    },
                );
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
