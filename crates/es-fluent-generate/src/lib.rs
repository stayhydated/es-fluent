#![doc = include_str!("../README.md")]

use clap::ValueEnum;
use es_fluent_core::meta::TypeKind;
use es_fluent_core::namer::FluentKey;
use es_fluent_core::registry::{FtlTypeInfo, FtlVariant};
use fluent_syntax::{ast, parser};
use std::collections::HashMap;
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

/// Generates a Fluent translation file from a list of `FtlTypeInfo` objects.
pub fn generate<P: AsRef<Path>>(
    crate_name: &str,
    i18n_path: P,
    items: Vec<FtlTypeInfo>,
    mode: FluentParseMode,
    dry_run: bool,
) -> Result<bool, FluentGenerateError> {
    let i18n_path = i18n_path.as_ref();

    if !dry_run {
        fs::create_dir_all(i18n_path)?;
    }

    let file_path = i18n_path.join(format!("{}.ftl", crate_name));

    let existing_resource = read_existing_resource(&file_path)?;

    let final_resource = if matches!(mode, FluentParseMode::Aggressive) {
        // In aggressive mode, completely replace with new content
        build_target_resource(&items)
    } else {
        // In conservative mode, merge with existing content
        smart_merge(existing_resource, &items, MergeBehavior::Append)
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
fn compare_type_infos(a: &FtlTypeInfo, b: &FtlTypeInfo) -> std::cmp::Ordering {
    // Infer is_this from variants
    let a_is_this = a
        .variants
        .iter()
        .any(|v| v.ftl_key.to_string().ends_with(FluentKey::THIS_SUFFIX));
    let b_is_this = b
        .variants
        .iter()
        .any(|v| v.ftl_key.to_string().ends_with(FluentKey::THIS_SUFFIX));

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
    items: &[FtlTypeInfo],
    behavior: MergeBehavior,
) -> ast::Resource<String> {
    let mut pending_items = merge_ftl_type_infos(items);
    pending_items.sort_by(compare_type_infos);

    let mut item_map: HashMap<String, FtlTypeInfo> = pending_items
        .into_iter()
        .map(|i| (i.type_name.clone(), i))
        .collect();

    let mut new_body = Vec::new();
    let mut current_group_name: Option<String> = None;
    let cleanup = matches!(behavior, MergeBehavior::Clean);

    for entry in existing.body {
        match entry {
            ast::Entry::GroupComment(ref comment) => {
                if let Some(ref old_group) = current_group_name
                    && let Some(info) = item_map.get_mut(old_group)
                    && !info.variants.is_empty()
                {
                    // Only append missing variants if we are appending
                    if matches!(behavior, MergeBehavior::Append) {
                        for variant in &info.variants {
                            new_body.push(create_message_entry(variant));
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
            ast::Entry::Message(ref msg) => {
                let key = &msg.id.name;
                let mut handled = false;

                if let Some(ref group_name) = current_group_name
                    && let Some(info) = item_map.get_mut(group_name)
                    && let Some(idx) = info
                        .variants
                        .iter()
                        .position(|v| v.ftl_key.to_string() == *key)
                {
                    info.variants.remove(idx);
                    handled = true;
                }

                if !handled {
                    for info in item_map.values_mut() {
                        if let Some(idx) = info
                            .variants
                            .iter()
                            .position(|v| v.ftl_key.to_string() == *key)
                        {
                            info.variants.remove(idx);
                            handled = true;
                            break;
                        }
                    }
                }

                if handled || !cleanup {
                    new_body.push(entry);
                }
            },
            ast::Entry::Term(ref term) => {
                let key = format!("{}{}", FluentKey::DELIMITER, term.id.name);
                let mut handled = false;
                for info in item_map.values_mut() {
                    if let Some(idx) = info
                        .variants
                        .iter()
                        .position(|v| v.ftl_key.to_string() == key)
                    {
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
        && !info.variants.is_empty()
    {
        // Only append missing variants if we are appending
        if matches!(behavior, MergeBehavior::Append) {
            for variant in &info.variants {
                new_body.push(create_message_entry(variant));
            }
        }
        info.variants.clear();
    }

    // Only append remaining new groups if we are appending
    if matches!(behavior, MergeBehavior::Append) {
        let mut remaining_groups: Vec<_> = item_map.into_iter().collect();
        remaining_groups.sort_by(|(_, a), (_, b)| compare_type_infos(a, b));

        for (type_name, info) in remaining_groups {
            if !info.variants.is_empty() {
                new_body.push(create_group_comment_entry(&type_name));
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

fn create_message_entry(variant: &FtlVariant) -> ast::Entry<String> {
    let message_id = ast::Identifier {
        name: variant.ftl_key.to_string(),
    };

    let base_value = ValueFormatter::expand(&variant.name);

    let mut elements = vec![ast::PatternElement::TextElement { value: base_value }];

    for arg_name in &variant.args {
        elements.push(ast::PatternElement::TextElement { value: " ".into() });

        elements.push(ast::PatternElement::Placeable {
            expression: ast::Expression::Inline(ast::InlineExpression::VariableReference {
                id: ast::Identifier {
                    name: arg_name.into(),
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

fn merge_ftl_type_infos(items: &[FtlTypeInfo]) -> Vec<FtlTypeInfo> {
    use std::collections::BTreeMap;

    // Group by type_name, also track module_path
    let mut grouped: BTreeMap<String, (TypeKind, Vec<FtlVariant>, String)> = BTreeMap::new();

    for item in items {
        let entry = grouped
            .entry(item.type_name.clone())
            .or_insert_with(|| (item.type_kind.clone(), Vec::new(), item.module_path.clone()));
        entry.1.extend(item.variants.clone());
    }

    grouped
        .into_iter()
        .map(|(type_name, (type_kind, mut variants, module_path))| {
            variants.sort_by(|a, b| {
                let a_is_this = a.ftl_key.to_string().ends_with(FluentKey::THIS_SUFFIX);
                let b_is_this = b.ftl_key.to_string().ends_with(FluentKey::THIS_SUFFIX);
                formatting::compare_with_this_priority(a_is_this, &a.name, b_is_this, &b.name)
            });
            variants.dedup();

            FtlTypeInfo {
                type_kind,
                type_name,
                variants,
                file_path: None,
                module_path,
            }
        })
        .collect()
}

fn build_target_resource(items: &[FtlTypeInfo]) -> ast::Resource<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_core::{meta::TypeKind, namer::FluentKey};
    use proc_macro2::Ident;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_value_formatter_expand() {
        assert_eq!(ValueFormatter::expand("simple-key"), "Key");
        assert_eq!(ValueFormatter::expand("another-test-value"), "Value");
        assert_eq!(ValueFormatter::expand("single"), "Single");
    }

    #[test]
    fn test_generate_empty_items() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![],
            FluentParseMode::Conservative,
            false,
        );
        assert!(result.is_ok());

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        assert!(!ftl_file_path.exists());
    }

    #[test]
    fn test_generate_with_items() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        let ftl_key = FluentKey::from(&Ident::new("TestEnum", proc_macro2::Span::call_site()))
            .join("Variant1");
        let variant = FtlVariant {
            name: "variant1".to_string(),
            ftl_key,
            args: Vec::new(),
            module_path: "test".to_string(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "TestEnum".to_string(),
            variants: vec![variant],
            file_path: None,
            module_path: "test".to_string(),
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![type_info],
            FluentParseMode::Conservative,
            false,
        );
        assert!(result.is_ok());

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        assert!(ftl_file_path.exists());

        let content = fs::read_to_string(ftl_file_path).unwrap();
        assert!(content.contains("TestEnum"));
        assert!(content.contains("Variant1"));
    }

    #[test]
    fn test_generate_aggressive_mode() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        fs::create_dir_all(&i18n_path).unwrap();
        fs::write(&ftl_file_path, "existing-message = Existing Content").unwrap();

        let ftl_key = FluentKey::from(&Ident::new("TestEnum", proc_macro2::Span::call_site()))
            .join("Variant1");
        let variant = FtlVariant {
            name: "variant1".to_string(),
            ftl_key,
            args: Vec::new(),
            module_path: "test".to_string(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "TestEnum".to_string(),
            variants: vec![variant],
            file_path: None,
            module_path: "test".to_string(),
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![type_info],
            FluentParseMode::Aggressive,
            false,
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(&ftl_file_path).unwrap();
        assert!(!content.contains("existing-message"));
        assert!(content.contains("TestEnum"));
        assert!(content.contains("Variant1"));
    }

    #[test]
    fn test_generate_conservative_mode() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        fs::create_dir_all(&i18n_path).unwrap();
        fs::write(&ftl_file_path, "existing-message = Existing Content").unwrap();

        let ftl_key = FluentKey::from(&Ident::new("TestEnum", proc_macro2::Span::call_site()))
            .join("Variant1");
        let variant = FtlVariant {
            name: "variant1".to_string(),
            ftl_key,
            args: Vec::new(),
            module_path: "test".to_string(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "TestEnum".to_string(),
            variants: vec![variant],
            file_path: None,
            module_path: "test".to_string(),
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![type_info],
            FluentParseMode::Conservative,
            false,
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(&ftl_file_path).unwrap();
        assert!(content.contains("existing-message"));
        assert!(content.contains("TestEnum"));
        assert!(content.contains("Variant1"));
    }
    #[test]
    fn test_generate_clean_mode() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        fs::create_dir_all(&i18n_path).unwrap();

        let initial_content = "
## OrphanGroup

what-Hi = Hi
awdawd = awdwa

## ExistingGroup

existing-key = Existing Value
";
        fs::write(&ftl_file_path, initial_content).unwrap();

        // Define items that match ExistingGroup but NOT OrphanGroup
        let ftl_key = FluentKey::from(&Ident::new("ExistingGroup", proc_macro2::Span::call_site()))
            .join("ExistingKey");
        let variant = FtlVariant {
            name: "ExistingKey".to_string(),
            ftl_key,
            args: Vec::new(),
            module_path: "test".to_string(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "ExistingGroup".to_string(),
            variants: vec![variant],
            file_path: None,
            module_path: "test".to_string(),
        };

        let result = crate::clean::clean("test_crate", &i18n_path, vec![type_info], false);
        assert!(result.is_ok());

        let content = fs::read_to_string(&ftl_file_path).unwrap();

        // Should NOT contain orphan content
        assert!(!content.contains("## OrphanGroup"));
        assert!(!content.contains("what-Hi"));
        assert!(!content.contains("awdawd"));

        // Should contain existing content that is still valid
        assert!(content.contains("## ExistingGroup"));
    }

    #[test]
    fn test_this_types_sorted_first() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        // Create types: Apple, Banana, Banana_this (should come first)
        let apple_variant = FtlVariant {
            name: "Red".to_string(),
            ftl_key: FluentKey::from(&Ident::new("Apple", proc_macro2::Span::call_site()))
                .join("Red"),
            args: Vec::new(),
            module_path: "test".to_string(),
        };
        let apple = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "Apple".to_string(),
            variants: vec![apple_variant],
            file_path: None,
            module_path: "test".to_string(),
        };

        let banana_variant = FtlVariant {
            name: "Yellow".to_string(),
            ftl_key: FluentKey::from(&Ident::new("Banana", proc_macro2::Span::call_site()))
                .join("Yellow"),
            args: Vec::new(),
            module_path: "test".to_string(),
        };
        let banana = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "Banana".to_string(),
            variants: vec![banana_variant],
            file_path: None,
            module_path: "test".to_string(),
        };

        // This type should come first despite alphabetical order
        // Use proper 'this' key suffix for inference!
        let banana_this_ident = Ident::new("BananaThis", proc_macro2::Span::call_site());
        let banana_this_key = FluentKey::new_this(&banana_this_ident); // "banana_this_this" effectively or similar

        let banana_this_variant = FtlVariant {
            name: "this".to_string(),
            ftl_key: banana_this_key,
            args: Vec::new(),
            module_path: "test".to_string(),
        };
        let banana_this = FtlTypeInfo {
            type_kind: TypeKind::Struct,
            type_name: "BananaThis".to_string(),
            variants: vec![banana_this_variant],
            file_path: None,
            module_path: "test".to_string(),
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![apple.clone(), banana.clone(), banana_this.clone()],
            FluentParseMode::Aggressive,
            false,
        );
        assert!(result.is_ok());

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        let content = fs::read_to_string(&ftl_file_path).unwrap();

        // BananaThis (is_this=true) should come before Apple and Banana
        let banana_this_pos = content.find("## BananaThis").expect("BananaThis missing");
        let apple_pos = content.find("## Apple").expect("Apple missing");
        let banana_pos = content.find("## Banana\n").expect("Banana missing");

        assert!(
            banana_this_pos < apple_pos,
            "BananaThis (is_this=true) should come before Apple"
        );
        assert!(
            banana_this_pos < banana_pos,
            "BananaThis (is_this=true) should come before Banana"
        );
        // Apple should come before Banana alphabetically
        assert!(apple_pos < banana_pos, "Apple should come before Banana");
    }

    #[test]
    fn test_this_variants_sorted_first_within_group() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        // Create a type with multiple variants, one being a "this" variant
        // Ensure keys have proper suffixes for inference
        let fruit_ident = Ident::new("Fruit", proc_macro2::Span::call_site());
        let this_key = FluentKey::new_this(&fruit_ident); // e.g. fruit_this

        let this_variant = FtlVariant {
            name: "this".to_string(),
            ftl_key: this_key,
            args: Vec::new(),
            module_path: "test".to_string(),
        };
        let apple_variant = FtlVariant {
            name: "Apple".to_string(),
            ftl_key: FluentKey::from(&fruit_ident).join("Apple"),
            args: Vec::new(),
            module_path: "test".to_string(),
        };
        let banana_variant = FtlVariant {
            name: "Banana".to_string(),
            ftl_key: FluentKey::from(&fruit_ident).join("Banana"),
            args: Vec::new(),
            module_path: "test".to_string(),
        };

        let fruit = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "Fruit".to_string(),
            // Deliberately put variants in wrong order
            variants: vec![
                banana_variant.clone(),
                this_variant.clone(),
                apple_variant.clone(),
            ],
            file_path: None,
            module_path: "test".to_string(),
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![fruit],
            FluentParseMode::Aggressive,
            false,
        );
        assert!(result.is_ok());

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        let content = fs::read_to_string(&ftl_file_path).unwrap();

        // The "this" variant (fruit) should come first, then Apple, then Banana
        let this_pos = content
            .find("fruit_this =")
            .expect("this variant (fruit_this) missing");
        let apple_pos = content.find("fruit-Apple").expect("Apple variant missing");
        let banana_pos = content
            .find("fruit-Banana")
            .expect("Banana variant missing");

        assert!(
            this_pos < apple_pos,
            "This variant should come before Apple"
        );
        assert!(
            this_pos < banana_pos,
            "This variant should come before Banana"
        );
        assert!(apple_pos < banana_pos, "Apple should come before Banana");
    }
}
