use es_fluent_core::namer::FluentKey;
use fluent_syntax::{ast, serializer};

/// Sort an FTL resource's entries alphabetically.
///
/// The sorting preserves group comments (## Comment) by organizing messages into sections.
/// Sections are sorted by their header, and messages within sections are sorted by key.
///
/// Additionally, this function attempts to "repair" grouping by matching message keys
/// to section headers (heuristically matching snake_case keys to PascalCase headers).
///
/// Keys ending in `_this` (defined by `FluentKey::THIS_SUFFIX`) are sorted to the top
/// of their respective sections.
pub fn sort_ftl_resource(resource: &ast::Resource<String>) -> String {
    #[derive(Debug, Default)]
    struct Section {
        /// The group comments (## Header) and any associated logic
        header: Vec<ast::Entry<String>>,
        /// The sort key derived from the header comment (e.g. "ButtonState")
        header_sort_key: String,
        /// Normalized sort key for matching (e.g. "buttonstate")
        matcher_key: String,
        /// Messages in this section
        messages: Vec<MessageEntry>,
    }

    #[derive(Debug)]
    struct MessageEntry {
        key: String,
        entries: Vec<ast::Entry<String>>,
        /// The index of the section this message was originally found in
        original_section_index: usize,
    }

    let mut sections: Vec<Section> = Vec::new();
    let mut current_section = Section::default();
    let mut current_comments: Vec<ast::Entry<String>> = Vec::new();

    // Helper to extract text from a GroupComment for sorting
    let get_group_name = |comment: &ast::Comment<String>| -> String {
        comment.content.iter().map(|s| s.trim()).collect::<String>()
    };

    // Helper to normalize strings for matching (remove non-alphanumeric, lowercase)
    let normalize = |s: &str| -> String {
        s
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect()
    };

    for entry in &resource.body {
        match entry {
            ast::Entry::GroupComment(comment) => {
                // Finish current section
                sections.push(std::mem::take(&mut current_section));
                
                // Start new section with this header
                current_section.header.push(entry.clone());
                let name = get_group_name(comment);
                current_section.header_sort_key = name.clone();
                current_section.matcher_key = normalize(&name);
                
                // Adopt pending comments
                if !current_comments.is_empty() {
                     let comments = std::mem::take(&mut current_comments);
                     current_section.header.splice(0..0, comments);
                }
            },
            ast::Entry::ResourceComment(_) => {
                // Resource comments go at the top (in the first section's header)
                current_section.header.push(entry.clone());
            },
            ast::Entry::Comment(_) => {
                // Accumulate comments
                current_comments.push(entry.clone());
            },
            ast::Entry::Message(msg) => {
                let key = msg.id.name.clone();
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                
                current_section.messages.push(MessageEntry {
                    key,
                    entries,
                    original_section_index: sections.len(),
                });
            },
            ast::Entry::Term(term) => {
                let key = format!("-{}", term.id.name);
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                
                current_section.messages.push(MessageEntry {
                    key,
                    entries,
                    original_section_index: sections.len(),
                });
            },
            ast::Entry::Junk { .. } => {
                // Skip junk entries
            },
        }
    }

    // Capture the last section
    sections.push(current_section);

    // Regrouping Pass
    let mut all_messages: Vec<MessageEntry> = Vec::new();
    for section in &mut sections {
        all_messages.append(&mut section.messages);
    }

    for msg in all_messages {
        let msg_clean = normalize(&msg.key);
        
        let mut best_score = 0;
        let mut best_section_idx = None;

        for (idx, section) in sections.iter().enumerate() {
            if section.matcher_key.is_empty() {
                continue;
            }
            if msg_clean.starts_with(&section.matcher_key) {
                let score = section.matcher_key.len();
                if score > best_score {
                    best_score = score;
                    best_section_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = best_section_idx {
            sections[idx].messages.push(msg);
        } else {
            // Restore to original section if possible, or first section
            if msg.original_section_index < sections.len() {
                sections[msg.original_section_index].messages.push(msg);
            } else {
                sections[0].messages.push(msg);
            }
        }
    }

    // Sort sections
    sections.sort_by(|a, b| {
        let a_key = &a.header_sort_key;
        let b_key = &b.header_sort_key;
        
        if a_key.is_empty() && b_key.is_empty() {
             std::cmp::Ordering::Equal 
        } else if a_key.is_empty() {
             std::cmp::Ordering::Less 
        } else if b_key.is_empty() {
             std::cmp::Ordering::Greater
        } else {
             a_key.cmp(b_key)
        }
    });

    // Sort messages within sections
    for section in &mut sections {
        section.messages.sort_by(|a, b| {
            // Check for _this suffix
            let a_is_this = a.key.ends_with(FluentKey::THIS_SUFFIX);
            let b_is_this = b.key.ends_with(FluentKey::THIS_SUFFIX);
            
            match (a_is_this, b_is_this) {
                (true, false) => std::cmp::Ordering::Less, // _this comes first
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.key.cmp(&b.key),
            }
        });
    }

    // Reconstruct
    let mut sorted_body: Vec<ast::Entry<String>> = Vec::new();
    
    for section in sections {
        sorted_body.extend(section.header);
        for msg in section.messages {
            sorted_body.extend(msg.entries);
        }
    }
    
    // Append any final trailing comments (rare)
    sorted_body.extend(current_comments);

    let sorted_resource = ast::Resource { body: sorted_body };
    serializer::serialize(&sorted_resource)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::parser;

    #[test]
    fn test_sort_ftl_simple() {
        let content = "zebra = Zebra\napple = Apple\nbanana = Banana";
        let resource = parser::parse(content.to_string()).unwrap();
        let sorted = sort_ftl_resource(&resource);

        // Messages should be sorted A-Z
        let lines: Vec<&str> = sorted.lines().collect();
        assert!(
            lines.iter().position(|l| l.starts_with("apple")).unwrap()
                < lines.iter().position(|l| l.starts_with("banana")).unwrap()
        );
        assert!(
            lines.iter().position(|l| l.starts_with("banana")).unwrap()
                < lines.iter().position(|l| l.starts_with("zebra")).unwrap()
        );
    }

    #[test]
    fn test_sort_ftl_with_group_comments() {
        let content = r#"## Zebras
zebra = Zebra

## Apples
apple = Apple"#;

        let resource = parser::parse(content.to_string()).unwrap();
        let sorted = sort_ftl_resource(&resource);

        // Apple group should come before Zebra group
        let apple_pos = sorted.find("## Apples").unwrap_or(usize::MAX);
        let zebra_pos = sorted.find("## Zebras").unwrap_or(usize::MAX);
        assert!(
            apple_pos < zebra_pos,
            "Apple group should come before Zebra group"
        );
    }

    #[test]
    fn test_sort_ftl_regrouping_dirty_input() {
        // "Dirty" input where `usa_state-A` is physically before `## USAState`.
        // The formatter should detect `usa_state...` matches `USAState` and move it.
        let content = r#"usa_state-A = A

## USAState
usa_state_this = Usa State"#;

        let resource = parser::parse(content.to_string()).unwrap();
        let sorted = sort_ftl_resource(&resource);

        println!("Sorted output:\n{}", sorted);

        let group_pos = sorted.find("## USAState").unwrap();
        let a_pos = sorted.find("usa_state-A = A").unwrap();
        let this_pos = sorted.find("usa_state_this = Usa State").unwrap();

        // Both messages must be AFTER the header
        assert!(a_pos > group_pos, "A should be moved after Group Header");
        assert!(this_pos > group_pos, "This should be after Group Header");
    }

    #[test]
    fn test_sort_ftl_prioritize_this() {
        let content = r#"## USAState
usa_state-A = A
usa_state_this = Usa State"#;

        let resource = parser::parse(content.to_string()).unwrap();
        let sorted = sort_ftl_resource(&resource);
        
        println!("Sorted output:\n{}", sorted);

        // _this should come BEFORE -A
        let a_pos = sorted.find("usa_state-A").unwrap();
        let this_pos = sorted.find("usa_state_this").unwrap();
        
        assert!(this_pos < a_pos, "_this should be sorted to top of group");
    }
}
