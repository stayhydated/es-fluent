//! FTL file parsing utilities.
//!
//! Provides shared functions for parsing FTL files and extracting
//! message information.

use anyhow::Result;
use fluent_syntax::{ast, parser};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Parse an FTL file and return the resource.
///
/// Returns an empty resource if the file doesn't exist or is empty.
/// Uses partial parse recovery for files with syntax errors.
pub fn parse_ftl_file(ftl_path: &Path) -> Result<ast::Resource<String>> {
    if !ftl_path.exists() {
        return Ok(ast::Resource { body: Vec::new() });
    }

    let content = fs::read_to_string(ftl_path)?;

    if content.trim().is_empty() {
        return Ok(ast::Resource { body: Vec::new() });
    }

    match parser::parse(content) {
        Ok(res) => Ok(res),
        Err((res, _)) => Ok(res), // Use partial result on parse errors
    }
}

/// Extract message keys from a resource.
pub fn extract_message_keys(resource: &ast::Resource<String>) -> HashSet<String> {
    resource
        .body
        .iter()
        .filter_map(|entry| {
            if let ast::Entry::Message(msg) = entry {
                Some(msg.id.name.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Extract variables from a message.
pub fn extract_variables_from_message(msg: &ast::Message<String>) -> HashSet<String> {
    let mut variables = HashSet::new();
    if let Some(ref value) = msg.value {
        extract_variables_from_pattern(value, &mut variables);
    }
    for attr in &msg.attributes {
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

/// Extract variables from an expression.
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

/// Extract variables from an inline expression.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ftl_file_nonexistent() {
        let result = parse_ftl_file(Path::new("/nonexistent/path.ftl")).unwrap();
        assert!(result.body.is_empty());
    }

    #[test]
    fn test_extract_message_keys() {
        let content = "hello = Hello\nworld = World";
        let resource = parser::parse(content.to_string()).unwrap();
        let keys = extract_message_keys(&resource);

        assert!(keys.contains("hello"));
        assert!(keys.contains("world"));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_extract_variables() {
        let content = "hello = Hello { $name }, you have { $count } messages";
        let resource = parser::parse(content.to_string()).unwrap();

        if let ast::Entry::Message(msg) = &resource.body[0] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("name"));
            assert!(vars.contains("count"));
            assert_eq!(vars.len(), 2);
        } else {
            panic!("Expected a message");
        }
    }

    #[test]
    fn test_extract_variables_from_select() {
        let content = r#"count = { $num ->
    [one] One item
   *[other] { $num } items
}"#;
        let resource = parser::parse(content.to_string()).unwrap();

        if let ast::Entry::Message(msg) = &resource.body[0] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("num"));
        } else {
            panic!("Expected a message");
        }
    }
}
