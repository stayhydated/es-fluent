#![doc = include_str!("../README.md")]

use clap::ValueEnum;
use es_fluent_derive_core::namer::FluentKey;
use es_fluent_derive_core::registry::{FtlTypeInfo, FtlVariant};
use fluent_syntax::{ast, parser};
use indexmap::IndexMap;
use std::{fs, path::Path};

pub mod clean;
pub mod error;
pub mod formatting;
pub mod value;

use error::FluentGenerateError;
use value::ValueFormatter;

/// The mode to use when parsing Fluent files.
#[derive(Clone, Debug, Default, PartialEq, ValueEnum)]
pub enum FluentParseMode {
    /// Overwrite existing translations.
    Aggressive,
    /// Preserve existing translations.
    #[default]
    Conservative,
}

impl std::fmt::Display for FluentParseMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aggressive => write!(f, "aggressive"),
            Self::Conservative => write!(f, "conservative"),
        }
    }
}

// Internal owned types for merge operations
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct OwnedVariant {
    name: String,
    ftl_key: String,
    args: Vec<String>,
}

impl From<&FtlVariant> for OwnedVariant {
    fn from(v: &FtlVariant) -> Self {
        Self {
            name: v.name.to_string(),
            ftl_key: v.ftl_key.to_string(),
            args: v.args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct OwnedTypeInfo {
    type_name: String,
    variants: Vec<OwnedVariant>,
}

impl From<&FtlTypeInfo> for OwnedTypeInfo {
    fn from(info: &FtlTypeInfo) -> Self {
        Self {
            type_name: info.type_name.to_string(),
            variants: info.variants.iter().map(OwnedVariant::from).collect(),
        }
    }
}

/// Generates a Fluent translation file from a list of `FtlTypeInfo` objects.
pub fn generate<P: AsRef<Path>, I: AsRef<FtlTypeInfo>>(
    crate_name: &str,
    i18n_path: P,
    items: &[I],
    mode: FluentParseMode,
    dry_run: bool,
) -> Result<bool, FluentGenerateError> {
    let i18n_path = i18n_path.as_ref();

    if !dry_run {
        fs::create_dir_all(i18n_path)?;
    }

    let file_path = i18n_path.join(format!("{}.ftl", crate_name));

    let existing_resource = read_existing_resource(&file_path)?;

    let items_ref: Vec<&FtlTypeInfo> = items.iter().map(|i| i.as_ref()).collect();

    let final_resource = if matches!(mode, FluentParseMode::Aggressive) {
        // In aggressive mode, completely replace with new content
        build_target_resource(&items_ref)
    } else {
        // In conservative mode, merge with existing content
        smart_merge(existing_resource, &items_ref, MergeBehavior::Append)
    };

    write_updated_resource(
        &file_path,
        &final_resource,
        dry_run,
        formatting::sort_ftl_resource,
    )
}

pub(crate) fn print_diff(old: &str, new: &str) {
    use colored::Colorize as _;
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{}", "  ...".dimmed());
        }
        for op in group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                let line = format!("{} {}", sign, change);
                match change.tag() {
                    ChangeTag::Delete => print!("{}", line.red()),
                    ChangeTag::Insert => print!("{}", line.green()),
                    ChangeTag::Equal => print!("{}", line.dimmed()),
                }
            }
        }
    }
}

/// Read and parse an existing FTL resource file.
///
/// Returns an empty resource if the file doesn't exist or is empty.
/// Logs warnings for parsing errors but continues with partial parse.
fn read_existing_resource(file_path: &Path) -> Result<ast::Resource<String>, FluentGenerateError> {
    if !file_path.exists() {
        return Ok(ast::Resource { body: Vec::new() });
    }

    let content = fs::read_to_string(file_path)?;
    if content.trim().is_empty() {
        return Ok(ast::Resource { body: Vec::new() });
    }

    match parser::parse(content) {
        Ok(res) => Ok(res),
        Err((res, errors)) => {
            tracing::warn!(
                "Warning: Encountered parsing errors in {}: {:?}",
                file_path.display(),
                errors
            );
            Ok(res)
        },
    }
}

/// Write an updated resource to disk, handling change detection and dry-run mode.
///
/// Returns `true` if the file was changed (or would be changed in dry-run mode).
fn write_updated_resource(
    file_path: &Path,
    resource: &ast::Resource<String>,
    dry_run: bool,
    formatter: impl Fn(&ast::Resource<String>) -> String,
) -> Result<bool, FluentGenerateError> {
    let is_empty = resource.body.is_empty();
    let final_content = if is_empty {
        String::new()
    } else {
        formatter(resource)
    };

    let current_content = if file_path.exists() {
        fs::read_to_string(file_path)?
    } else {
        String::new()
    };

    // Determine if content has changed
    let has_changed = match is_empty {
        true => current_content != final_content && !current_content.trim().is_empty(),
        false => current_content.trim() != final_content.trim(),
    };

    if !has_changed {
        log_unchanged(file_path, is_empty, dry_run);
        return Ok(false);
    }

    write_or_preview(
        file_path,
        &current_content,
        &final_content,
        is_empty,
        dry_run,
    )?;
    Ok(true)
}

/// Log that a file was unchanged (only when not in dry-run mode).
fn log_unchanged(file_path: &Path, is_empty: bool, dry_run: bool) {
    if dry_run {
        return;
    }
    let msg = match is_empty {
        true => format!(
            "FTL file unchanged (empty or no items): {}",
            file_path.display()
        ),
        false => format!("FTL file unchanged: {}", file_path.display()),
    };
    tracing::debug!("{}", msg);
}

/// Write changes to disk or preview them in dry-run mode.
fn write_or_preview(
    file_path: &Path,
    current_content: &str,
    final_content: &str,
    is_empty: bool,
    dry_run: bool,
) -> Result<(), FluentGenerateError> {
    if dry_run {
        let display_path = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
        let msg = match (is_empty, !current_content.trim().is_empty()) {
            (true, true) => format!(
                "Would write empty FTL file (no items): {}",
                display_path.display()
            ),
            (true, false) => format!("Would write empty FTL file: {}", display_path.display()),
            (false, _) => format!("Would update FTL file: {}", display_path.display()),
        };
        println!("{}", msg);
        print_diff(current_content, final_content);
        println!();
        return Ok(());
    }

    fs::write(file_path, final_content)?;
    let msg = match is_empty {
        true => format!("Wrote empty FTL file (no items): {}", file_path.display()),
        false => format!("Updated FTL file: {}", file_path.display()),
    };
    tracing::info!("{}", msg);
    Ok(())
}

/// Compares two type infos, putting "this" types first.
fn compare_type_infos(a: &OwnedTypeInfo, b: &OwnedTypeInfo) -> std::cmp::Ordering {
    // Infer is_this from variants
    let a_is_this = a
        .variants
        .iter()
        .any(|v| v.ftl_key.ends_with(FluentKey::THIS_SUFFIX));
    let b_is_this = b
        .variants
        .iter()
        .any(|v| v.ftl_key.ends_with(FluentKey::THIS_SUFFIX));

    formatting::compare_with_this_priority(a_is_this, &a.type_name, b_is_this, &b.type_name)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MergeBehavior {
    /// Add new keys and preserve existing ones.
    Append,
    /// Remove orphan keys and empty groups, do not add new keys.
    Clean,
}

pub(crate) fn smart_merge(
    existing: ast::Resource<String>,
    items: &[&FtlTypeInfo],
    behavior: MergeBehavior,
) -> ast::Resource<String> {
    let mut pending_items = merge_ftl_type_infos(items);
    pending_items.sort_by(compare_type_infos);

    let mut item_map: IndexMap<String, OwnedTypeInfo> = pending_items
        .into_iter()
        .map(|i| (i.type_name.clone(), i))
        .collect();
    let mut key_to_group: IndexMap<String, String> = IndexMap::new();
    for (group_name, info) in &item_map {
        for variant in &info.variants {
            key_to_group.insert(variant.ftl_key.clone(), group_name.clone());
        }
    }
    let mut relocated_by_group: IndexMap<String, Vec<ast::Entry<String>>> = IndexMap::new();

    let mut new_body = Vec::new();
    let mut current_group_name: Option<String> = None;
    let cleanup = matches!(behavior, MergeBehavior::Clean);

    for entry in existing.body {
        match entry {
            ast::Entry::GroupComment(ref comment) => {
                if let Some(ref old_group) = current_group_name
                    && let Some(info) = item_map.get_mut(old_group)
                {
                    // Only append missing variants if we are appending
                    if matches!(behavior, MergeBehavior::Append) {
                        if let Some(entries) = relocated_by_group.shift_remove(old_group) {
                            new_body.extend(entries);
                        }
                        if !info.variants.is_empty() {
                            for variant in &info.variants {
                                new_body.push(create_message_entry(variant));
                            }
                        }
                    }
                    info.variants.clear();
                }

                if let Some(content) = comment.content.first() {
                    let trimmed = content.trim();
                    current_group_name = Some(trimmed.to_string());
                } else {
                    current_group_name = None;
                }

                let keep_group = if let Some(ref group_name) = current_group_name {
                    !cleanup || item_map.contains_key(group_name)
                } else {
                    true
                };

                if keep_group {
                    new_body.push(entry);
                }
            },
            ast::Entry::Message(msg) => {
                let key = msg.id.name.clone();
                let mut handled = false;
                let mut relocate_to: Option<String> = None;

                if let Some(ref group_name) = current_group_name
                    && let Some(info) = item_map.get_mut(group_name)
                    && let Some(idx) = info.variants.iter().position(|v| v.ftl_key == key)
                {
                    info.variants.remove(idx);
                    handled = true;
                }

                if !handled
                    && let Some(expected_group) = key_to_group.get(&key)
                    && matches!(behavior, MergeBehavior::Append)
                    && current_group_name.as_deref() != Some(expected_group.as_str())
                    && let Some(info) = item_map.get_mut(expected_group)
                    && let Some(idx) = info.variants.iter().position(|v| v.ftl_key == key)
                {
                    info.variants.remove(idx);
                    relocate_to = Some(expected_group.clone());
                }

                if relocate_to.is_none() && !handled {
                    for info in item_map.values_mut() {
                        if let Some(idx) = info.variants.iter().position(|v| v.ftl_key == key) {
                            info.variants.remove(idx);
                            handled = true;
                            break;
                        }
                    }
                }

                if let Some(group_name) = relocate_to {
                    relocated_by_group
                        .entry(group_name)
                        .or_default()
                        .push(ast::Entry::Message(msg));
                } else if handled || !cleanup {
                    new_body.push(ast::Entry::Message(msg));
                }
            },
            ast::Entry::Term(ref term) => {
                let key = format!("{}{}", FluentKey::DELIMITER, term.id.name);
                let mut handled = false;
                for info in item_map.values_mut() {
                    if let Some(idx) = info.variants.iter().position(|v| v.ftl_key == key) {
                        info.variants.remove(idx);
                        handled = true;
                        break;
                    }
                }

                if handled || !cleanup {
                    new_body.push(entry);
                }
            },
            ast::Entry::Junk { .. } => {
                new_body.push(entry);
            },
            _ => {
                new_body.push(entry);
            },
        }
    }

    // Correctly handle the end of the last group
    if let Some(ref last_group) = current_group_name
        && let Some(info) = item_map.get_mut(last_group)
    {
        // Only append missing variants if we are appending
        if matches!(behavior, MergeBehavior::Append) {
            if let Some(entries) = relocated_by_group.shift_remove(last_group) {
                new_body.extend(entries);
            }
            if !info.variants.is_empty() {
                for variant in &info.variants {
                    new_body.push(create_message_entry(variant));
                }
            }
        }
        info.variants.clear();
    }

    // Only append remaining new groups if we are appending
    if matches!(behavior, MergeBehavior::Append) {
        let mut remaining_groups: Vec<_> = item_map.into_iter().collect();
        remaining_groups.sort_by(|(_, a), (_, b)| compare_type_infos(a, b));

        for (type_name, info) in remaining_groups {
            let relocated = relocated_by_group.shift_remove(&type_name);
            if !info.variants.is_empty() || relocated.is_some() {
                new_body.push(create_group_comment_entry(&type_name));
                if let Some(entries) = relocated {
                    new_body.extend(entries);
                }
                for variant in info.variants {
                    new_body.push(create_message_entry(&variant));
                }
            }
        }
    }

    ast::Resource { body: new_body }
}

fn create_group_comment_entry(type_name: &str) -> ast::Entry<String> {
    ast::Entry::GroupComment(ast::Comment {
        content: vec![type_name.to_owned()],
    })
}

fn create_message_entry(variant: &OwnedVariant) -> ast::Entry<String> {
    let message_id = ast::Identifier {
        name: variant.ftl_key.clone(),
    };

    let base_value = ValueFormatter::expand(&variant.name);

    let mut elements = vec![ast::PatternElement::TextElement { value: base_value }];

    for arg_name in &variant.args {
        elements.push(ast::PatternElement::TextElement { value: " ".into() });

        elements.push(ast::PatternElement::Placeable {
            expression: ast::Expression::Inline(ast::InlineExpression::VariableReference {
                id: ast::Identifier {
                    name: arg_name.clone(),
                },
            }),
        });
    }

    let pattern = ast::Pattern { elements };

    ast::Entry::Message(ast::Message {
        id: message_id,
        value: Some(pattern),
        attributes: Vec::new(),
        comment: None,
    })
}

fn merge_ftl_type_infos(items: &[&FtlTypeInfo]) -> Vec<OwnedTypeInfo> {
    use std::collections::BTreeMap;

    // Group by type_name
    let mut grouped: BTreeMap<String, Vec<OwnedVariant>> = BTreeMap::new();

    for item in items {
        let entry = grouped.entry(item.type_name.to_string()).or_default();
        entry.extend(item.variants.iter().map(OwnedVariant::from));
    }

    grouped
        .into_iter()
        .map(|(type_name, mut variants)| {
            variants.sort_by(|a, b| {
                let a_is_this = a.ftl_key.ends_with(FluentKey::THIS_SUFFIX);
                let b_is_this = b.ftl_key.ends_with(FluentKey::THIS_SUFFIX);
                formatting::compare_with_this_priority(a_is_this, &a.name, b_is_this, &b.name)
            });
            variants.dedup();

            OwnedTypeInfo {
                type_name,
                variants,
            }
        })
        .collect()
}

fn build_target_resource(items: &[&FtlTypeInfo]) -> ast::Resource<String> {
    let items = merge_ftl_type_infos(items);
    let mut body: Vec<ast::Entry<String>> = Vec::new();
    let mut sorted_items = items.to_vec();
    sorted_items.sort_by(compare_type_infos);

    for info in &sorted_items {
        body.push(create_group_comment_entry(&info.type_name));

        for variant in &info.variants {
            body.push(create_message_entry(variant));
        }
    }

    ast::Resource { body }
}
