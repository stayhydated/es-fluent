use es_fluent_generate::ftl::{entry_key, group_comment_name, is_section_comment};
use fluent_syntax::ast;
use indexmap::IndexMap;
use std::collections::HashSet;

type EntryBundle = Vec<ast::Entry<String>>;

/// Classification of an FTL entry for merge operations.
enum EntryKind<'a> {
    /// Group or resource comment (section header).
    SectionComment,
    /// Regular comment.
    Comment,
    /// Message with key.
    Message(std::borrow::Cow<'a, str>),
    /// Term with key (prefixed with -).
    Term(std::borrow::Cow<'a, str>),
    /// Junk or other entries.
    Other,
}

/// Classify an FTL entry for merge operations.
fn classify_entry(entry: &ast::Entry<String>) -> EntryKind<'_> {
    if is_section_comment(entry) {
        return EntryKind::SectionComment;
    }

    if matches!(entry, ast::Entry::Comment(_)) {
        return EntryKind::Comment;
    }

    if let Some(key) = entry_key(entry) {
        return match entry {
            ast::Entry::Message(_) => EntryKind::Message(key),
            ast::Entry::Term(_) => EntryKind::Term(key),
            _ => EntryKind::Other,
        };
    }

    EntryKind::Other
}

/// Merge missing keys from the fallback into the existing resource.
pub(super) fn merge_missing_keys(
    existing: &ast::Resource<String>,
    fallback: &ast::Resource<String>,
    missing_keys: &[&String],
    added_keys: &mut Vec<String>,
) -> ast::Resource<String> {
    let missing_set: HashSet<&str> = missing_keys.iter().map(|key| key.as_str()).collect();
    let existing_groups = collect_group_comments(existing);
    let mut pending_by_group =
        collect_missing_entry_bundles(&existing_groups, fallback, &missing_set, added_keys);
    let pending_entry_count = pending_by_group
        .values()
        .flat_map(|bundles| bundles.iter())
        .map(Vec::len)
        .sum::<usize>();
    let mut body: Vec<ast::Entry<String>> =
        Vec::with_capacity(existing.body.len() + pending_entry_count);
    let mut current_group: Option<String> = None;

    for entry in &existing.body {
        if let ast::Entry::GroupComment(comment) = entry {
            extend_group_bundles(&mut body, &mut pending_by_group, current_group.take());
            current_group = group_comment_name(comment);
        }

        body.push(entry.clone());
    }

    extend_group_bundles(&mut body, &mut pending_by_group, current_group.take());

    for (_group, bundles) in pending_by_group {
        for bundle in bundles {
            body.extend(bundle);
        }
    }

    ast::Resource { body }
}

fn collect_missing_entry_bundles(
    existing_groups: &HashSet<String>,
    fallback: &ast::Resource<String>,
    missing_set: &HashSet<&str>,
    added_keys: &mut Vec<String>,
) -> IndexMap<Option<String>, Vec<EntryBundle>> {
    let mut bundles_by_group: IndexMap<Option<String>, Vec<EntryBundle>> = IndexMap::new();
    let mut inserted_groups: HashSet<String> = HashSet::new();
    let mut fallback_comments: Vec<ast::Entry<String>> = Vec::new();
    let mut current_group: Option<String> = None;

    for entry in &fallback.body {
        match classify_entry(entry) {
            EntryKind::SectionComment => {
                if let ast::Entry::GroupComment(comment) = entry {
                    current_group = group_comment_name(comment);
                    fallback_comments.clear();
                    let keep_group = current_group.as_ref().is_none_or(|name| {
                        !existing_groups.contains(name) && !inserted_groups.contains(name)
                    });
                    if keep_group {
                        fallback_comments.push(entry.clone());
                    }
                }
            },
            EntryKind::Comment => {
                fallback_comments.push(entry.clone());
            },
            EntryKind::Message(key) | EntryKind::Term(key) => {
                if missing_set.contains(key.as_ref()) {
                    added_keys.push(key.to_string());
                    let mut bundle = std::mem::take(&mut fallback_comments);
                    bundle.push(entry.clone());
                    for bundle_entry in &bundle {
                        if let ast::Entry::GroupComment(comment) = bundle_entry
                            && let Some(name) = group_comment_name(comment)
                        {
                            inserted_groups.insert(name);
                        }
                    }
                    bundles_by_group
                        .entry(current_group.clone())
                        .or_default()
                        .push(bundle);
                } else {
                    fallback_comments.clear();
                }
            },
            EntryKind::Other => {},
        }
    }

    bundles_by_group
}

fn extend_group_bundles(
    body: &mut Vec<ast::Entry<String>>,
    pending_by_group: &mut IndexMap<Option<String>, Vec<EntryBundle>>,
    group_name: Option<String>,
) {
    if let Some(bundles) = pending_by_group.shift_remove(&group_name) {
        for bundle in bundles {
            body.extend(bundle);
        }
    }
}

fn collect_group_comments(resource: &ast::Resource<String>) -> HashSet<String> {
    let mut groups = HashSet::new();
    for entry in &resource.body {
        if let ast::Entry::GroupComment(comment) = entry
            && let Some(name) = group_comment_name(comment)
        {
            groups.insert(name);
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::{parser, serializer};

    #[test]
    fn classify_entry_covers_comment_term_and_other() {
        let parsed =
            parser::parse("## Group\n# Note\n-term = value\nmessage = Value\n".to_string())
                .unwrap();
        let comment_entry = ast::Entry::Comment(ast::Comment {
            content: vec!["inline".to_string()],
        });

        assert!(
            parsed
                .body
                .iter()
                .any(|entry| matches!(classify_entry(entry), EntryKind::SectionComment))
        );
        assert!(matches!(classify_entry(&comment_entry), EntryKind::Comment));
        assert!(
            parsed.body.iter().any(
                |entry| matches!(classify_entry(entry), EntryKind::Term(key) if key == "-term")
            )
        );
        assert!(parsed.body.iter().any(
            |entry| matches!(classify_entry(entry), EntryKind::Message(key) if key == "message")
        ));

        let (partial, _) = parser::parse("broken = { $x\n".to_string()).unwrap_err();
        assert!(
            partial
                .body
                .iter()
                .any(|entry| matches!(classify_entry(entry), EntryKind::Other))
        );
    }

    #[test]
    fn group_comment_helpers_handle_empty_and_non_empty_content() {
        let parsed =
            parser::parse("## Header Name\n\nkey = Value\n##    \nother = Value\n".to_string())
                .unwrap();

        let first = match &parsed.body[0] {
            ast::Entry::GroupComment(comment) => comment,
            _ => panic!("expected group comment"),
        };
        assert_eq!(group_comment_name(first).as_deref(), Some("Header Name"));

        let second = parsed
            .body
            .iter()
            .find_map(|entry| match entry {
                ast::Entry::GroupComment(comment)
                    if comment
                        .content
                        .first()
                        .is_some_and(|line| line.trim().is_empty()) =>
                {
                    Some(comment)
                },
                _ => None,
            })
            .expect("blank group comment");
        assert!(group_comment_name(second).is_none());

        let groups = collect_group_comments(&parsed);
        assert!(groups.contains("Header Name"));
    }

    #[test]
    fn merge_missing_keys_merges_terms_and_resets_fallback_comments_for_non_missing_entries() {
        let existing = parser::parse("hello = Hello\n".to_string()).unwrap();
        let fallback = parser::parse(
            "## Group\n# carry\npresent = Present\n# drop-me\n-brand = Brand\n".to_string(),
        )
        .unwrap();

        let term = "-brand".to_string();
        let missing_keys: Vec<&String> = vec![&term];
        let mut added = Vec::new();

        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);
        let content = serializer::serialize(&merged);

        assert_eq!(added, vec!["-brand".to_string()]);
        assert!(content.contains("-brand = Brand"));
        assert!(
            !content.contains("# carry"),
            "comments before non-missing key should be cleared before merge"
        );
    }

    #[test]
    fn merge_missing_keys_covers_comment_term_resource_comment_and_other_entries() {
        let mut existing =
            parser::parse("existing = Existing\n-existing_term = Existing Term\n".to_string())
                .unwrap();
        existing.body.insert(
            0,
            ast::Entry::Comment(ast::Comment {
                content: vec!["existing-comment".to_string()],
            }),
        );

        let (junk_resource, _) = parser::parse("broken = { $x\n".to_string()).unwrap_err();
        let junk = junk_resource.body.first().cloned().expect("junk entry");
        existing.body.push(junk.clone());

        let mut fallback = parser::parse(
            "### Resource Header\n## NewGroup\n# fallback-comment\nnew = New\n-new_term = New Term\n"
                .to_string(),
        )
        .unwrap();
        fallback.body.push(junk);

        let new_msg = "new".to_string();
        let new_term = "-new_term".to_string();
        let missing_keys: Vec<&String> = vec![&new_msg, &new_term];
        let mut added = Vec::new();
        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);
        let content = serializer::serialize(&merged);

        assert!(added.contains(&"new".to_string()));
        assert!(added.contains(&"-new_term".to_string()));
        assert!(content.contains("new = New"));
        assert!(content.contains("-new_term = New Term"));
    }

    #[test]
    fn test_merge_missing_keys() {
        let existing_content = "hello = Hello";
        let fallback_content = "hello = Hello\nworld = World\ngoodbye = Goodbye";

        let existing = parser::parse(existing_content.to_string()).unwrap();
        let fallback = parser::parse(fallback_content.to_string()).unwrap();

        let world = "world".to_string();
        let goodbye = "goodbye".to_string();
        let missing_keys: Vec<&String> = vec![&world, &goodbye];
        let mut added = Vec::new();

        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);

        assert_eq!(added.len(), 2);
        assert!(added.contains(&"world".to_string()));
        assert!(added.contains(&"goodbye".to_string()));

        // The merged resource should have all 3 messages
        let merged_keys = crate::ftl::extract_message_keys(&merged);
        assert_eq!(merged_keys.len(), 3);
    }

    #[test]
    fn test_merge_missing_keys_skips_duplicate_group_comments() {
        let existing_content = r#"## CountryLabelVariants

country_label_variants-Canada = Canada
"#;
        let fallback_content = r#"## CountryLabelVariants

country_label_variants-Canada = Canada
country_label_variants-USA = Usa
"#;

        let existing = parser::parse(existing_content.to_string()).unwrap();
        let fallback = parser::parse(fallback_content.to_string()).unwrap();

        let usa = "country_label_variants-USA".to_string();
        let missing_keys: Vec<&String> = vec![&usa];
        let mut added = Vec::new();

        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);

        let content = serializer::serialize(&merged);
        assert!(
            content.contains("country_label_variants-USA"),
            "Missing key should be merged"
        );
        assert_eq!(
            content.matches("## CountryLabelVariants").count(),
            1,
            "Group comment should not be duplicated: {content}"
        );
    }

    #[test]
    fn merge_missing_keys_preserves_existing_key_order() {
        let existing = parser::parse("beta = Beta\nalpha = Alpha\n".to_string()).unwrap();
        let fallback =
            parser::parse("beta = Beta\naardvark = Aardvark\nalpha = Alpha\n".to_string()).unwrap();

        let aardvark = "aardvark".to_string();
        let missing_keys: Vec<&String> = vec![&aardvark];
        let mut added = Vec::new();
        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);

        let ordered_keys: Vec<String> = merged
            .body
            .iter()
            .filter_map(|entry| match entry {
                ast::Entry::Message(message) => Some(message.id.name.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(added, vec!["aardvark".to_string()]);
        assert_eq!(
            ordered_keys,
            vec![
                "beta".to_string(),
                "alpha".to_string(),
                "aardvark".to_string(),
            ]
        );
    }

    #[test]
    fn merge_missing_keys_inserts_missing_keys_at_existing_group_tail() {
        let existing =
            parser::parse("## Alpha\nalpha = Alpha\n\n## Beta\nbeta = Beta\n".to_string()).unwrap();
        let fallback = parser::parse(
            "## Alpha\nalpha = Alpha\nalpha_two = Alpha Two\n\n## Beta\nbeta = Beta\n".to_string(),
        )
        .unwrap();

        let alpha_two = "alpha_two".to_string();
        let missing_keys: Vec<&String> = vec![&alpha_two];
        let mut added = Vec::new();
        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);
        let content = serializer::serialize(&merged);

        assert_eq!(added, vec!["alpha_two".to_string()]);
        assert!(
            content
                .find("alpha_two = Alpha Two")
                .expect("missing key inserted")
                < content.find("## Beta").expect("beta group present"),
            "missing key should be inserted before the next group header: {content}"
        );
    }
}
