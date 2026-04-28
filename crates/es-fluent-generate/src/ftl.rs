use fluent_syntax::{ast, parser};
use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

fn empty_resource() -> ast::Resource<String> {
    ast::Resource { body: Vec::new() }
}

/// Render parser errors into a short, user-facing string.
pub fn format_parse_errors(errors: &[fluent_syntax::parser::ParserError]) -> String {
    let preview: Vec<String> = errors
        .iter()
        .take(3)
        .map(|error| format!("{error:?}"))
        .collect();

    if errors.len() > preview.len() {
        format!(
            "{}; ... and {} more",
            preview.join("; "),
            errors.len() - preview.len()
        )
    } else {
        preview.join("; ")
    }
}

/// Parse raw FTL content, returning a partial resource plus any parse errors.
pub fn parse_ftl_content(
    content: String,
) -> (
    ast::Resource<String>,
    Vec<fluent_syntax::parser::ParserError>,
) {
    if content.trim().is_empty() {
        return (empty_resource(), Vec::new());
    }

    match parser::parse(content) {
        Ok(resource) => (resource, Vec::new()),
        Err((resource, errors)) => (resource, errors),
    }
}

/// Parse an FTL file and return the resource plus any parse errors.
pub fn parse_ftl_file_with_errors(
    ftl_path: &Path,
) -> std::io::Result<(
    ast::Resource<String>,
    Vec<fluent_syntax::parser::ParserError>,
)> {
    if !ftl_path.exists() {
        return Ok((empty_resource(), Vec::new()));
    }

    if ftl_path.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Expected FTL file path, found directory: {}",
                ftl_path.display()
            ),
        ));
    }

    let content = fs::read_to_string(ftl_path)?;
    Ok(parse_ftl_content(content))
}

/// Parse an FTL file and reject parser errors.
pub fn parse_ftl_file(ftl_path: &Path) -> std::io::Result<ast::Resource<String>> {
    let (resource, errors) = parse_ftl_file_with_errors(ftl_path)?;
    if errors.is_empty() {
        Ok(resource)
    } else {
        Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Refusing to use '{}' because it contains Fluent parse errors: {}",
                ftl_path.display(),
                format_parse_errors(&errors)
            ),
        ))
    }
}

/// Extract message keys from a resource.
pub fn extract_message_keys(resource: &ast::Resource<String>) -> HashSet<String> {
    resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::Message(msg) => Some(msg.id.name.clone()),
            _ => None,
        })
        .collect()
}

/// Extract variables from a message.
pub fn extract_variables_from_message(msg: &ast::Message<String>) -> HashSet<String> {
    extract_variables_from_value_and_attributes(msg.value.as_ref(), &msg.attributes)
}

/// Extract variables from a value pattern plus attributes.
pub fn extract_variables_from_value_and_attributes(
    value: Option<&ast::Pattern<String>>,
    attributes: &[ast::Attribute<String>],
) -> HashSet<String> {
    let mut variables = HashSet::new();
    if let Some(pattern) = value {
        extract_variables_from_pattern(pattern, &mut variables);
    }
    for attr in attributes {
        extract_variables_from_pattern(&attr.value, &mut variables);
    }
    variables
}

/// Extract variables from a pattern.
pub fn extract_variables_from_pattern(
    pattern: &ast::Pattern<String>,
    variables: &mut HashSet<String>,
) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            extract_variables_from_expression(expression, variables);
        }
    }
}

fn extract_variables_from_expression(
    expression: &ast::Expression<String>,
    variables: &mut HashSet<String>,
) {
    match expression {
        ast::Expression::Inline(inline) => {
            extract_variables_from_inline(inline, variables);
        },
        ast::Expression::Select { selector, variants } => {
            extract_variables_from_inline(selector, variables);
            for variant in variants {
                extract_variables_from_pattern(&variant.value, variables);
            }
        },
    }
}

fn extract_variables_from_inline(
    inline: &ast::InlineExpression<String>,
    variables: &mut HashSet<String>,
) {
    match inline {
        ast::InlineExpression::VariableReference { id } => {
            variables.insert(id.name.clone());
        },
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for arg in &arguments.positional {
                extract_variables_from_inline(arg, variables);
            }
            for arg in &arguments.named {
                extract_variables_from_inline(&arg.value, variables);
            }
        },
        ast::InlineExpression::Placeable { expression } => {
            extract_variables_from_expression(expression, variables);
        },
        _ => {},
    }
}

/// Extract the stable key for a message or term entry.
pub fn entry_key(entry: &ast::Entry<String>) -> Option<Cow<'_, str>> {
    match entry {
        ast::Entry::Message(msg) => Some(Cow::Borrowed(&msg.id.name)),
        ast::Entry::Term(term) => Some(Cow::Owned(format!("-{}", term.id.name))),
        _ => None,
    }
}

/// Returns true when an entry is a section-level comment.
pub fn is_section_comment(entry: &ast::Entry<String>) -> bool {
    matches!(
        entry,
        ast::Entry::GroupComment(_) | ast::Entry::ResourceComment(_)
    )
}

/// Extract the normalized display name from a group comment.
pub fn group_comment_name(comment: &ast::Comment<String>) -> Option<String> {
    comment
        .content
        .first()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_ftl_file_nonexistent() {
        let result = parse_ftl_file(Path::new("/nonexistent/path.ftl")).unwrap();
        assert!(result.body.is_empty());
    }

    #[test]
    fn parse_ftl_content_empty_and_partial_recovery() {
        let (empty, empty_errors) = parse_ftl_content("   \n".to_string());
        assert!(empty.body.is_empty());
        assert!(empty_errors.is_empty());

        let (partial, errors) = parse_ftl_content("hello = { $name\nworld = World\n".to_string());
        assert!(!partial.body.is_empty());
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_ftl_file_rejects_parse_errors() {
        let temp = tempdir().expect("tempdir");
        let file_path = temp.path().join("invalid.ftl");
        std::fs::write(&file_path, "broken = {\n").expect("write invalid");

        let err = parse_ftl_file(&file_path).expect_err("expected parse error");
        assert!(err.to_string().contains("Refusing to use"));
        assert!(err.to_string().contains("Fluent parse errors"));
    }

    #[test]
    fn parse_ftl_file_errors_when_path_is_directory() {
        let temp = tempdir().expect("tempdir");
        let dir_path = temp.path().join("not-a-file");
        std::fs::create_dir_all(&dir_path).expect("create dir");

        let err = parse_ftl_file(&dir_path).expect_err("expected io error");
        assert!(
            err.to_string().contains("Is a directory") || err.to_string().contains("directory")
        );
    }

    #[test]
    fn extract_message_keys_ignores_non_message_entries() {
        let resource = parser::parse("-term = Value\n# Comment\n".to_string()).unwrap();
        let keys = extract_message_keys(&resource);
        assert!(keys.is_empty());
    }

    #[test]
    fn extract_variables_cover_values_attributes_and_functions() {
        let resource = parser::parse(
            r#"hello = Hello { $name }, you have { $count } messages
count = { $num ->
    [one] One item
   *[other] { $num } items
}
msg = { FUNC($direct, named: 1) }
    .attr = Attr { $attr }
nested = { { $wrapped } }"#
                .to_string(),
        )
        .unwrap();

        if let ast::Entry::Message(msg) = &resource.body[0] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("name"));
            assert!(vars.contains("count"));
        } else {
            panic!("Expected a message");
        }

        if let ast::Entry::Message(msg) = &resource.body[1] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("num"));
        } else {
            panic!("Expected a message");
        }

        if let ast::Entry::Message(msg) = &resource.body[2] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("direct"));
            assert!(vars.contains("attr"));
        } else {
            panic!("Expected a message");
        }

        if let ast::Entry::Message(msg) = &resource.body[3] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("wrapped"));
        } else {
            panic!("Expected a message");
        }
    }

    #[test]
    fn entry_helpers_cover_comments_terms_and_messages() {
        let parsed =
            parser::parse("## Group\n# Note\n-term = value\nmessage = Value\n".to_string())
                .unwrap();
        let first_group = match &parsed.body[0] {
            ast::Entry::GroupComment(comment) => comment,
            _ => panic!("expected group comment"),
        };

        assert!(is_section_comment(&parsed.body[0]));
        assert_eq!(group_comment_name(first_group).as_deref(), Some("Group"));
        let keys = parsed
            .body
            .iter()
            .filter_map(|entry| entry_key(entry).map(|key| key.into_owned()))
            .collect::<Vec<_>>();
        assert!(keys.contains(&"-term".to_string()));
        assert!(keys.contains(&"message".to_string()));
        assert!(entry_key(&parsed.body[0]).is_none());
    }
}
