use es_fluent_generate::ftl::{entry_key, group_comment_name, is_section_comment};
use fluent_syntax::ast;
use std::collections::{BTreeMap, HashSet};

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
    let missing_set: HashSet<&String> = missing_keys.iter().copied().collect();
    let existing_groups = collect_group_comments(existing);
    let mut inserted_groups: HashSet<String> = HashSet::new();

    // Group existing entries by key for preservation
    let mut entries_by_key: BTreeMap<String, Vec<ast::Entry<String>>> = BTreeMap::new();
    let mut standalone_comments: Vec<ast::Entry<String>> = Vec::new();
    let mut current_comments: Vec<ast::Entry<String>> = Vec::new();

    // Process existing entries
    for entry in &existing.body {
        match classify_entry(entry) {
            EntryKind::SectionComment => {
                standalone_comments.append(&mut current_comments);
                current_comments.push(entry.clone());
            },
            EntryKind::Comment => {
                current_comments.push(entry.clone());
            },
            EntryKind::Message(key) => {
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                entries_by_key.insert(key.to_string(), entries);
            },
            EntryKind::Term(key) => {
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                entries_by_key.insert(key.to_string(), entries);
            },
            EntryKind::Other => {},
        }
    }

    // Add missing entries from fallback
    let mut fallback_comments: Vec<ast::Entry<String>> = Vec::new();

    for entry in &fallback.body {
        match classify_entry(entry) {
            EntryKind::SectionComment => {
                // ResourceComment is skipped in original, GroupComment starts fresh
                if let ast::Entry::GroupComment(comment) = entry {
                    fallback_comments.clear();
                    let group_name = group_comment_name(comment);
                    let keep_group = group_name.as_ref().is_none_or(|name| {
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
                let key_str = key.to_string();
                if missing_set.contains(&key_str) {
                    added_keys.push(key_str.clone());
                    let mut entries = std::mem::take(&mut fallback_comments);
                    entries.push(entry.clone());
                    for entry in &entries {
                        if let ast::Entry::GroupComment(comment) = entry
                            && let Some(name) = group_comment_name(comment)
                        {
                            inserted_groups.insert(name);
                        }
                    }
                    entries_by_key.insert(key_str, entries);
                } else {
                    fallback_comments.clear();
                }
            },
            EntryKind::Other => {},
        }
    }

    // Build sorted body
    let mut body: Vec<ast::Entry<String>> = Vec::new();
    body.extend(standalone_comments);
    body.append(&mut current_comments);

    for (_key, entries) in entries_by_key {
        body.extend(entries);
    }

    ast::Resource { body }
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
}
