use super::context::ValidationContext;
use crate::core::ValidationIssue;
use crate::ftl::{LoadedFtlFile, extract_variables_from_message};
use fluent_syntax::ast;
use indexmap::IndexMap;
use std::collections::HashSet;

#[derive(Clone)]
struct ActualKeyInfo {
    variables: HashSet<String>,
    file_path: String,
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
        let Some(actual) = actual_keys.get(key) else {
            let fallback = first_actual_file(&actual_keys)
                .unwrap_or_else(|| ("unknown.ftl".to_string(), "unknown.ftl".to_string()));
            issues.push(ctx.missing_key_issue(key, locale, &fallback.0, &fallback.1));
            continue;
        };

        for variable in &key_info.variables {
            if actual.variables.contains(variable) {
                continue;
            }

            issues.push(ctx.missing_variable_issue(
                key,
                variable,
                locale,
                &actual.header_link,
                key_info.source_file.as_deref(),
                key_info.source_line,
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
) -> IndexMap<String, ActualKeyInfo> {
    let mut actual_keys: IndexMap<String, ActualKeyInfo> = IndexMap::new();

    for file in loaded_files {
        let relative_path = ctx.to_relative_path(&file.abs_path);
        let header_link = ctx.format_terminal_link(
            &relative_path,
            &format!("file://{}", file.abs_path.display()),
        );

        for entry in &file.resource.body {
            if let ast::Entry::Message(msg) = entry {
                let key = msg.id.name.clone();
                if let Some(previous) = actual_keys.get(&key) {
                    issues.push(ctx.duplicate_key_issue(
                        &key,
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
                        variables: extract_variables_from_message(msg),
                        file_path: relative_path.clone(),
                        header_link: header_link.clone(),
                    },
                );
            }
        }
    }

    actual_keys
}

fn first_actual_file(actual_keys: &IndexMap<String, ActualKeyInfo>) -> Option<(String, String)> {
    actual_keys
        .values()
        .next()
        .map(|info| (info.file_path.clone(), info.header_link.clone()))
}
