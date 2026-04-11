use crate::ast_build::{create_group_comment_entry, create_message_entry};
use crate::model::{compare_type_infos, merge_ftl_type_infos};
use es_fluent_derive_core::namer::FluentKey;
use es_fluent_derive_core::registry::FtlTypeInfo;
use fluent_syntax::ast;
use indexmap::IndexMap;
use std::collections::HashSet;

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

    let mut item_map: IndexMap<String, _> = pending_items
        .into_iter()
        .map(|info| (info.type_name.clone(), info))
        .collect();
    let mut key_to_group: IndexMap<String, String> = IndexMap::new();
    for (group_name, info) in &item_map {
        for variant in &info.variants {
            key_to_group.insert(variant.ftl_key.clone(), group_name.clone());
        }
    }
    let mut relocated_by_group: IndexMap<String, Vec<ast::Entry<String>>> = IndexMap::new();
    let mut late_relocated_by_group: IndexMap<String, Vec<ast::Entry<String>>> = IndexMap::new();
    let mut seen_groups: HashSet<String> = HashSet::new();
    let existing_keys = collect_existing_keys(&existing);
    let mut seen_keys: HashSet<String> = HashSet::new();

    let mut new_body = Vec::new();
    let mut current_group_name: Option<String> = None;
    let cleanup = matches!(behavior, MergeBehavior::Clean);

    for entry in existing.body {
        match entry {
            ast::Entry::GroupComment(ref comment) => {
                if let Some(ref old_group) = current_group_name
                    && let Some(info) = item_map.get_mut(old_group)
                {
                    if matches!(behavior, MergeBehavior::Append) {
                        if let Some(entries) = relocated_by_group.shift_remove(old_group) {
                            new_body.extend(entries);
                        }
                        if !info.variants.is_empty() {
                            for variant in &info.variants {
                                if !existing_keys.contains(&variant.ftl_key) {
                                    seen_keys.insert(variant.ftl_key.clone());
                                    new_body.push(create_message_entry(variant));
                                }
                            }
                        }
                    }
                    info.variants.clear();
                }

                current_group_name = comment
                    .content
                    .first()
                    .map(|content| content.trim())
                    .filter(|content| !content.is_empty())
                    .map(ToOwned::to_owned);

                let keep_group = if let Some(ref group_name) = current_group_name {
                    !cleanup || item_map.contains_key(group_name)
                } else {
                    true
                };

                if keep_group {
                    new_body.push(entry);
                }

                if let Some(ref group_name) = current_group_name {
                    seen_groups.insert(group_name.clone());
                }
            },
            ast::Entry::Message(msg) => {
                let key = msg.id.name.clone();
                let mut handled = false;
                let mut relocate_to: Option<String> = None;

                if seen_keys.contains(&key) {
                    continue;
                }

                if let Some(expected_group) = key_to_group.get(&key).cloned() {
                    if current_group_name.as_deref() != Some(expected_group.as_str())
                        && matches!(behavior, MergeBehavior::Append)
                    {
                        relocate_to = Some(expected_group.clone());
                    }
                    handled = true;

                    if let Some(info) = item_map.get_mut(&expected_group)
                        && let Some(idx) = info.variants.iter().position(|v| v.ftl_key == key)
                    {
                        info.variants.remove(idx);
                    }
                } else if !handled {
                    for info in item_map.values_mut() {
                        if let Some(idx) = info.variants.iter().position(|v| v.ftl_key == key) {
                            info.variants.remove(idx);
                            handled = true;
                            break;
                        }
                    }
                }

                if let Some(group_name) = relocate_to {
                    seen_keys.insert(key);
                    if seen_groups.contains(&group_name) {
                        late_relocated_by_group
                            .entry(group_name)
                            .or_default()
                            .push(ast::Entry::Message(msg));
                    } else {
                        relocated_by_group
                            .entry(group_name)
                            .or_default()
                            .push(ast::Entry::Message(msg));
                    }
                } else if handled || !cleanup {
                    seen_keys.insert(key);
                    new_body.push(ast::Entry::Message(msg));
                }
            },
            ast::Entry::Term(ref term) => {
                let key = format!("{}{}", FluentKey::DELIMITER, term.id.name);
                let mut handled = false;
                if seen_keys.contains(&key) {
                    continue;
                }
                for info in item_map.values_mut() {
                    if let Some(idx) = info.variants.iter().position(|v| v.ftl_key == key) {
                        info.variants.remove(idx);
                        handled = true;
                        break;
                    }
                }

                if handled || !cleanup {
                    seen_keys.insert(key);
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

    if let Some(ref last_group) = current_group_name
        && let Some(info) = item_map.get_mut(last_group)
    {
        if matches!(behavior, MergeBehavior::Append) {
            if let Some(entries) = relocated_by_group.shift_remove(last_group) {
                new_body.extend(entries);
            }
            if !info.variants.is_empty() {
                for variant in &info.variants {
                    if !existing_keys.contains(&variant.ftl_key) {
                        seen_keys.insert(variant.ftl_key.clone());
                        new_body.push(create_message_entry(variant));
                    }
                }
            }
        }
        info.variants.clear();
    }

    if matches!(behavior, MergeBehavior::Append) {
        let mut remaining_groups: Vec<_> = item_map.into_iter().collect();
        remaining_groups.sort_by(|(_, a), (_, b)| compare_type_infos(a, b));

        for (type_name, info) in remaining_groups {
            let relocated = relocated_by_group.shift_remove(&type_name);
            let has_missing = info
                .variants
                .iter()
                .any(|variant| !existing_keys.contains(&variant.ftl_key));
            if has_missing || relocated.is_some() {
                new_body.push(create_group_comment_entry(&type_name));
                if let Some(entries) = relocated {
                    new_body.extend(entries);
                }
                for variant in info.variants {
                    if !existing_keys.contains(&variant.ftl_key) {
                        seen_keys.insert(variant.ftl_key.clone());
                        new_body.push(create_message_entry(&variant));
                    }
                }
            }
        }
    }

    let mut resource = ast::Resource { body: new_body };

    if matches!(behavior, MergeBehavior::Append) && !late_relocated_by_group.is_empty() {
        insert_late_relocated(&mut resource.body, &late_relocated_by_group);
    }
    if cleanup {
        remove_empty_group_comments(resource)
    } else {
        resource
    }
}

pub(crate) fn group_comment_name(comment: &ast::Comment<String>) -> Option<String> {
    comment
        .content
        .first()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
}

pub(crate) fn collect_existing_keys(resource: &ast::Resource<String>) -> HashSet<String> {
    let mut keys = HashSet::new();
    for entry in &resource.body {
        match entry {
            ast::Entry::Message(msg) => {
                keys.insert(msg.id.name.clone());
            },
            ast::Entry::Term(term) => {
                keys.insert(format!("{}{}", FluentKey::DELIMITER, term.id.name));
            },
            _ => {},
        }
    }
    keys
}

pub(crate) fn insert_late_relocated(
    body: &mut Vec<ast::Entry<String>>,
    late_relocated_by_group: &IndexMap<String, Vec<ast::Entry<String>>>,
) {
    let mut group_positions: Vec<(String, usize)> = Vec::new();
    for (idx, entry) in body.iter().enumerate() {
        if let ast::Entry::GroupComment(comment) = entry
            && let Some(name) = group_comment_name(comment)
        {
            group_positions.push((name, idx));
        }
    }

    if group_positions.is_empty() {
        return;
    }

    let mut inserted: HashSet<String> = HashSet::new();
    for (i, (name, _start)) in group_positions.iter().enumerate().rev() {
        if inserted.contains(name) {
            continue;
        }
        let end = if i + 1 < group_positions.len() {
            group_positions[i + 1].1
        } else {
            body.len()
        };
        if let Some(entries) = late_relocated_by_group.get(name)
            && !entries.is_empty()
        {
            body.splice(end..end, entries.clone());
        }
        inserted.insert(name.clone());
    }
}

pub(crate) fn remove_empty_group_comments(
    resource: ast::Resource<String>,
) -> ast::Resource<String> {
    let mut body: Vec<ast::Entry<String>> = Vec::with_capacity(resource.body.len());
    let mut pending_group: Option<ast::Entry<String>> = None;
    let mut pending_entries: Vec<ast::Entry<String>> = Vec::new();
    let mut has_message = false;

    let flush_pending = |body: &mut Vec<ast::Entry<String>>,
                         pending_group: &mut Option<ast::Entry<String>>,
                         pending_entries: &mut Vec<ast::Entry<String>>,
                         has_message: &mut bool| {
        if let Some(group_comment) = pending_group.take() {
            if *has_message {
                body.push(group_comment);
            }
            body.append(pending_entries);
        }
        *has_message = false;
    };

    for entry in resource.body {
        match entry {
            ast::Entry::GroupComment(_) => {
                flush_pending(
                    &mut body,
                    &mut pending_group,
                    &mut pending_entries,
                    &mut has_message,
                );
                pending_group = Some(entry);
                pending_entries = Vec::new();
            },
            ast::Entry::Message(_) | ast::Entry::Term(_) => {
                if pending_group.is_some() {
                    has_message = true;
                    pending_entries.push(entry);
                } else {
                    body.push(entry);
                }
            },
            _ => {
                if pending_group.is_some() {
                    pending_entries.push(entry);
                } else {
                    body.push(entry);
                }
            },
        }
    }

    flush_pending(
        &mut body,
        &mut pending_group,
        &mut pending_entries,
        &mut has_message,
    );

    ast::Resource { body }
}
