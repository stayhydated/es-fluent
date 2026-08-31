use super::validation::{validate_tree_locale_dir, validate_tree_locale_setup};

use crate::ftl::LocaleContext;

use anyhow::Result;

use fluent_syntax::ast;

use serde::Serialize;

use std::path::Path;

#[derive(Serialize)]
pub(super) struct TreeJsonReport {
    pub(super) crates: Vec<TreeCrateJson>,
    pub(super) error_count: usize,
    pub(super) errors: Vec<TreeErrorJson>,
}

#[derive(Serialize)]
pub(super) struct TreeCrateJson {
    pub(super) name: String,
    pub(super) locales: Vec<TreeLocaleJson>,
}

#[derive(Serialize)]
pub(super) struct TreeLocaleJson {
    pub(super) locale: String,
    pub(super) files: Vec<TreeFileJson>,
}

#[derive(Serialize)]
pub(super) struct TreeErrorJson {
    pub(super) crate_name: String,
    pub(super) message: String,
}

#[derive(Serialize)]
pub(super) struct TreeFileJson {
    pub(super) path: String,
    pub(super) parse_error: bool,
    pub(super) entries: Vec<TreeEntryJson>,
}

#[derive(Serialize)]
pub(super) struct TreeEntryJson {
    pub(super) id: String,
    pub(super) kind: &'static str,
    pub(super) attributes: Vec<String>,
    pub(super) variables: Vec<String>,
}
pub(super) fn build_crate_tree_json(
    krate: &crate::core::CrateInfo,
    all_locales: bool,
    include_attributes: bool,
    include_variables: bool,
) -> Result<(TreeCrateJson, Vec<String>)> {
    let ctx = LocaleContext::from_crate(krate, all_locales)?;
    validate_tree_locale_setup(&ctx, all_locales)?;
    let mut locales = Vec::new();
    let mut parse_errors = Vec::new();

    for locale in &ctx.locales {
        let locale_dir = ctx.locale_dir(locale);
        validate_tree_locale_dir(locale, &locale_dir)?;

        let ftl_files = ctx.discover_files(locale)?;
        let mut files = Vec::new();
        for file_info in &ftl_files {
            let relative_path = crate::utils::paths::slash_path(&file_info.relative_path);
            let (file, parse_error) = build_file_tree_json(
                &relative_path,
                &file_info.abs_path,
                include_attributes,
                include_variables,
            );
            files.push(file);
            if let Some(error) = parse_error {
                parse_errors.push(format!(
                    "failed to parse FTL file '{relative_path}': {error}"
                ));
            }
        }

        locales.push(TreeLocaleJson {
            locale: locale.clone(),
            files,
        });
    }

    Ok((
        TreeCrateJson {
            name: krate.name.to_string(),
            locales,
        },
        parse_errors,
    ))
}

pub(super) fn build_file_tree_json(
    relative_path: &str,
    abs_path: &Path,
    include_attributes: bool,
    include_variables: bool,
) -> (TreeFileJson, Option<String>) {
    let resource = match crate::ftl::parse_ftl_file(abs_path) {
        Ok(resource) => resource,
        Err(error) => {
            return (
                TreeFileJson {
                    path: relative_path.to_string(),
                    parse_error: true,
                    entries: Vec::new(),
                },
                Some(error.to_string()),
            );
        },
    };

    let entries = resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::Message(message) => Some(TreeEntryJson {
                id: message.id.name.clone(),
                kind: "message",
                attributes: if include_attributes {
                    message
                        .attributes
                        .iter()
                        .map(|attribute| attribute.id.name.clone())
                        .collect()
                } else {
                    Vec::new()
                },
                variables: if include_variables {
                    let attributes = if include_attributes {
                        message.attributes.as_slice()
                    } else {
                        &[]
                    };
                    let mut variables = crate::ftl::extract_variables_from_value_and_attributes(
                        message.value.as_ref(),
                        attributes,
                    )
                    .into_iter()
                    .collect::<Vec<_>>();
                    variables.sort();
                    variables
                } else {
                    Vec::new()
                },
            }),
            ast::Entry::Term(term) => Some(TreeEntryJson {
                id: format!("-{}", term.id.name),
                kind: "term",
                attributes: if include_attributes {
                    term.attributes
                        .iter()
                        .map(|attribute| attribute.id.name.clone())
                        .collect()
                } else {
                    Vec::new()
                },
                variables: if include_variables {
                    let attributes = if include_attributes {
                        term.attributes.as_slice()
                    } else {
                        &[]
                    };
                    let mut variables = crate::ftl::extract_variables_from_value_and_attributes(
                        Some(&term.value),
                        attributes,
                    )
                    .into_iter()
                    .collect::<Vec<_>>();
                    variables.sort();
                    variables
                } else {
                    Vec::new()
                },
            }),
            _ => None,
        })
        .collect();

    (
        TreeFileJson {
            path: relative_path.to_string(),
            parse_error: false,
            entries,
        },
        None,
    )
}
