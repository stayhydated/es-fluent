use clap::ValueEnum;
use es_fluent_core::meta::TypeKind;
use es_fluent_core::registry::{FtlTypeInfo, FtlVariant};
use fluent_syntax::{ast, parser, serializer};
use std::collections::HashMap;
use std::{fs, path::Path};

pub mod error;
mod formatter;

use error::FluentGenerateError;
use formatter::value::ValueFormatter;

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum FluentParseMode {
    Aggressive,
    #[default]
    Conservative,
}

pub fn generate<P: AsRef<Path>>(
    crate_name: String,
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
                    log::error!(
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

    let target_resource = build_target_resource(&items);

    let mut existing_entries_map: HashMap<String, ast::Entry<String>> = HashMap::new();
    for entry in existing_resource.body.into_iter() {
        match &entry {
            ast::Entry::Message(msg) => {
                existing_entries_map.insert(msg.id.name.clone(), entry);
            },
            ast::Entry::Term(term) => {
                existing_entries_map.insert(format!("-{}", term.id.name), entry);
            },
            _ => {},
        }
    }

    let mut merged_resource_body: Vec<ast::Entry<String>> = Vec::new();
    let mut keys_added_or_preserved = HashMap::new();

    for entry in target_resource.body {
        match entry {
            ast::Entry::GroupComment(_) => {
                merged_resource_body.push(entry);
            },
            ast::Entry::Message(msg) => {
                let key = msg.id.name.clone();
                let entry_to_add = if matches!(mode, FluentParseMode::Aggressive) {
                    ast::Entry::Message(msg)
                } else {
                    existing_entries_map
                        .remove(&key)
                        .unwrap_or(ast::Entry::Message(msg))
                };
                merged_resource_body.push(entry_to_add);
                keys_added_or_preserved.insert(key, ());
            },
            ast::Entry::Term(term) => {
                let key = format!("-{}", term.id.name);
                let entry_to_add = if matches!(mode, FluentParseMode::Aggressive) {
                    ast::Entry::Term(term)
                } else {
                    existing_entries_map
                        .remove(&key)
                        .unwrap_or(ast::Entry::Term(term))
                };
                merged_resource_body.push(entry_to_add);
                keys_added_or_preserved.insert(key, ());
            },
            _ => {
                merged_resource_body.push(entry);
            },
        }
    }

    if matches!(mode, FluentParseMode::Conservative) {
        for (_, entry) in existing_entries_map {
            merged_resource_body.push(entry);
        }
    }

    let final_resource = ast::Resource {
        body: merged_resource_body,
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
        } else if !file_path.exists() {
            log::error!(
                "FTL file remains non-existent (no items): {}",
                file_path.display()
            );
        } else {
            log::error!(
                "FTL file unchanged (empty or no items): {}",
                file_path.display()
            );
        }
    }

    Ok(())
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

    if let Some(args) = &variant.arguments {
        for arg_name in args {
            elements.push(ast::PatternElement::TextElement { value: " ".into() });

            elements.push(ast::PatternElement::Placeable {
                expression: ast::Expression::Inline(ast::InlineExpression::VariableReference {
                    id: ast::Identifier {
                        name: arg_name.into(),
                    },
                }),
            });
        }
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

    // Group by `type_name`
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
                let a_is_this = !a.ftl_key.to_string().contains('-');
                let b_is_this = !b.ftl_key.to_string().contains('-');

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
