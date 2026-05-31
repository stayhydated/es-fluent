use crate::model::{OwnedVariant, compare_type_infos};
use crate::value::ValueFormatter;
use es_fluent_shared::EsFluentResult;
use es_fluent_shared::registry::FtlTypeInfo;
use fluent_syntax::ast;

/// Create a group comment entry for a type section.
pub(crate) fn create_group_comment_entry(type_name: &str) -> ast::Entry<String> {
    ast::Entry::GroupComment(ast::Comment {
        content: vec![type_name.to_owned()],
    })
}

/// Create a message entry from an owned variant definition.
pub(crate) fn create_message_entry(variant: &OwnedVariant) -> ast::Entry<String> {
    let message_id = ast::Identifier {
        name: variant.ftl_key().to_string(),
    };

    let base_value = ValueFormatter::expand(&variant.name);
    let mut elements = vec![ast::PatternElement::TextElement { value: base_value }];

    for arg_name in &variant.args {
        elements.push(ast::PatternElement::TextElement { value: " ".into() });
        elements.push(ast::PatternElement::Placeable {
            expression: ast::Expression::Inline(ast::InlineExpression::VariableReference {
                id: ast::Identifier {
                    name: arg_name.to_string(),
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

/// Build a full target resource from the current registered type infos.
pub(crate) fn build_target_resource(
    items: &[&FtlTypeInfo],
) -> EsFluentResult<ast::Resource<String>> {
    let items = crate::model::merge_ftl_type_infos(items)?;
    let mut body: Vec<ast::Entry<String>> = Vec::new();
    let mut sorted_items = items.to_vec();
    sorted_items.sort_by(compare_type_infos);

    for info in &sorted_items {
        body.push(create_group_comment_entry(&info.type_name));

        for variant in &info.variants {
            body.push(create_message_entry(variant));
        }
    }

    Ok(ast::Resource { body })
}
