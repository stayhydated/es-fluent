#![doc = include_str!("../README.md")]

use clap::ValueEnum;
use es_fluent_core::meta::TypeKind;
use es_fluent_core::namer::FluentKey;
use es_fluent_core::registry::{FtlTypeInfo, FtlVariant};
use fluent_syntax::{ast, parser, serializer};
use std::collections::HashMap;
use std::{fs, path::Path};

pub mod error;
mod formatter;

use error::FluentGenerateError;
use formatter::value::ValueFormatter;

/// The mode to use when parsing Fluent files.
#[derive(Clone, Debug, Default, PartialEq, ValueEnum)]
pub enum FluentParseMode {
    /// Overwrite existing translations.
    Aggressive,
    /// Preserve existing translations.
    #[default]
    Conservative,
    /// Clean orphan keys (unused in code) but preserve used translations.
    #[clap(skip)]
    Clean,
}

/// Generates a Fluent translation file from a list of `FtlTypeInfo` objects.
pub fn generate<P: AsRef<Path>>(
    crate_name: &str,
    i18n_path: P,
    items: Vec<FtlTypeInfo>,
    mode: FluentParseMode,
) -> Result<(), FluentGenerateError> {
    let i18n_path = i18n_path.as_ref();

    fs::create_dir_all(i18n_path)?;

    let file_path = i18n_path.join(format!("{}.ftl", crate_name));

    let existing_resource = if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        if content.trim().is_empty() {
            ast::Resource { body: Vec::new() }
        } else {
            match parser::parse(content) {
                Ok(res) => res,
                Err((res, errors)) => {
                    log::warn!(
                        "Warning: Encountered parsing errors in {}: {:?}",
                        file_path.display(),
                        errors
                    );
                    res
                },
            }
        }
    } else {
        ast::Resource { body: Vec::new() }
    };

    let final_resource = if matches!(mode, FluentParseMode::Aggressive) {
        let target_resource = build_target_resource(&items);

        let mut existing_entries_map: HashMap<String, ast::Entry<String>> = HashMap::new();
        for entry in existing_resource.body.into_iter() {
            match &entry {
                ast::Entry::Message(msg) => {
                    existing_entries_map.insert(msg.id.name.clone(), entry);
                },
                ast::Entry::Term(term) => {
                    existing_entries_map
                        .insert(format!("{}{}", FluentKey::DELIMITER, term.id.name), entry);
                },
                _ => {},
            }
        }

        let mut merged_resource_body: Vec<ast::Entry<String>> = Vec::new();

        for entry in target_resource.body {
            merged_resource_body.push(entry);
        }

        ast::Resource {
            body: merged_resource_body,
        }
    } else {
        let cleanup = matches!(mode, FluentParseMode::Clean);
        smart_merge(existing_resource, &items, cleanup)
    };

    if !final_resource.body.is_empty() {
        let final_output = serializer::serialize(&final_resource);

        let final_content_to_write = final_output.trim_end();

        let current_content = if file_path.exists() {
            fs::read_to_string(&file_path)?
        } else {
            String::new()
        };

        if current_content != final_content_to_write {
            fs::write(&file_path, final_content_to_write)?;
            log::error!("Updated FTL file: {}", file_path.display());
        } else {
            log::error!("FTL file unchanged: {}", file_path.display());
        }
    } else {
        let final_content_to_write = "".to_string();
        let current_content = if file_path.exists() {
            fs::read_to_string(&file_path)?
        } else {
            String::new()
        };

        if current_content != final_content_to_write && !current_content.trim().is_empty() {
            fs::write(&file_path, &final_content_to_write)?;
            log::error!("Wrote empty FTL file (no items): {}", file_path.display());
        } else {
            if current_content != final_content_to_write {
                fs::write(&file_path, &final_content_to_write)?;
            }
            log::error!(
                "FTL file unchanged (empty or no items): {}",
                file_path.display()
            );
        }
    }

    Ok(())
}

fn smart_merge(
    existing: ast::Resource<String>,
    items: &[FtlTypeInfo],
    cleanup: bool,
) -> ast::Resource<String> {
    let mut pending_items = merge_ftl_type_infos(items);
    pending_items.sort_by(|a, b| a.type_name.cmp(&b.type_name));

    let mut item_map: HashMap<String, FtlTypeInfo> = pending_items
        .into_iter()
        .map(|i| (i.type_name.clone(), i))
        .collect();

    let mut new_body = Vec::new();
    let mut current_group_name: Option<String> = None;

    for entry in existing.body {
        match entry {
            ast::Entry::GroupComment(ref comment) => {
                if let Some(ref old_group) = current_group_name {
                    if let Some(info) = item_map.get_mut(old_group) {
                        if !info.variants.is_empty() {
                            for variant in &info.variants {
                                new_body.push(create_message_entry(variant));
                            }
                            info.variants.clear();
                        }
                    }
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

                if let Some(ref group_name) = current_group_name {
                    if let Some(info) = item_map.get_mut(group_name) {
                        if let Some(idx) = info
                            .variants
                            .iter()
                            .position(|v| v.ftl_key.to_string() == *key)
                        {
                            info.variants.remove(idx);
                            handled = true;
                        }
                    }
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
    if let Some(ref last_group) = current_group_name {
        if let Some(info) = item_map.get_mut(last_group) {
            if !info.variants.is_empty() {
                for variant in &info.variants {
                    new_body.push(create_message_entry(variant));
                }
                info.variants.clear();
            }
        }
    }

    let mut remaining_groups: Vec<_> = item_map.into_iter().collect();
    remaining_groups.sort_by(|(na, _), (nb, _)| na.cmp(nb));

    for (type_name, info) in remaining_groups {
        if !info.variants.is_empty() {
            new_body.push(create_group_comment_entry(&type_name));
            for variant in info.variants {
                new_body.push(create_message_entry(&variant));
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

    // Group by type_name
    let mut grouped: BTreeMap<String, (TypeKind, Vec<FtlVariant>)> = BTreeMap::new();

    for item in items {
        let entry = grouped
            .entry(item.type_name.clone())
            .or_insert_with(|| (item.type_kind.clone(), Vec::new()));
        entry.1.extend(item.variants.clone());
    }

    grouped
        .into_iter()
        .map(|(type_name, (type_kind, mut variants))| {
            variants.sort_by(|a, b| {
                // Put "this" variants (those without a dash in the key) first
                let a_is_this = !a.ftl_key.to_string().contains(FluentKey::DELIMITER);
                let b_is_this = !b.ftl_key.to_string().contains(FluentKey::DELIMITER);

                match (a_is_this, b_is_this) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });
            variants.dedup();

            FtlTypeInfo {
                type_kind,
                type_name,
                variants,
                file_path: None,
            }
        })
        .collect()
}

fn build_target_resource(items: &[FtlTypeInfo]) -> ast::Resource<String> {
    let items = merge_ftl_type_infos(items);
    let mut body: Vec<ast::Entry<String>> = Vec::new();
    let mut sorted_items = items.to_vec();
    sorted_items.sort_by(|a, b| a.type_name.cmp(&b.type_name));

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
        );
        assert!(result.is_ok());

        let ftl_file_path = i18n_path.join("test_crate.ftl");
        assert!(!ftl_file_path.exists());
    }

    #[test]
    fn test_generate_with_items() {
        let temp_dir = TempDir::new().unwrap();
        let i18n_path = temp_dir.path().join("i18n");

        let ftl_key = FluentKey::new(
            &Ident::new("TestEnum", proc_macro2::Span::call_site()),
            "Variant1",
        );
        let variant = FtlVariant {
            name: "variant1".to_string(),
            ftl_key,
            args: Vec::new(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "TestEnum".to_string(),
            variants: vec![variant],
            file_path: None,
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![type_info],
            FluentParseMode::Conservative,
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

        let ftl_key = FluentKey::new(
            &Ident::new("TestEnum", proc_macro2::Span::call_site()),
            "Variant1",
        );
        let variant = FtlVariant {
            name: "variant1".to_string(),
            ftl_key,
            args: Vec::new(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "TestEnum".to_string(),
            variants: vec![variant],
            file_path: None,
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![type_info],
            FluentParseMode::Aggressive,
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

        let ftl_key = FluentKey::new(
            &Ident::new("TestEnum", proc_macro2::Span::call_site()),
            "Variant1",
        );
        let variant = FtlVariant {
            name: "variant1".to_string(),
            ftl_key,
            args: Vec::new(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "TestEnum".to_string(),
            variants: vec![variant],
            file_path: None,
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![type_info],
            FluentParseMode::Conservative,
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
        let ftl_key = FluentKey::new(
            &Ident::new("ExistingGroup", proc_macro2::Span::call_site()),
            "ExistingKey",
        );
        let variant = FtlVariant {
            name: "ExistingKey".to_string(),
            ftl_key,
            args: Vec::new(),
        };

        let type_info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "ExistingGroup".to_string(),
            variants: vec![variant],
            file_path: None,
        };

        let result = generate(
            "test_crate",
            &i18n_path,
            vec![type_info],
            FluentParseMode::Clean,
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(&ftl_file_path).unwrap();

        // Should NOT contain orphan content
        assert!(!content.contains("## OrphanGroup"));
        assert!(!content.contains("what-Hi"));
        assert!(!content.contains("awdawd"));

        // Should contain existing content that is still valid
        assert!(content.contains("## ExistingGroup"));
    }
}
