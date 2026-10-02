use es_fluent_generate::{formatting::sort_ftl_resource, ftl::extract_variables_from_message};
use fluent_syntax::{ast, parser, serializer};
use proptest::prelude::*;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

fn parse(source: String) -> ast::Resource<String> {
    parser::parse(source).expect("generated FTL must remain valid while shrinking")
}

fn payloads(resource: &ast::Resource<String>) -> BTreeMap<String, ast::Entry<String>> {
    resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::Message(message) => Some((message.id.name.clone(), entry.clone())),
            ast::Entry::Term(term) => Some((format!("-{}", term.id.name), entry.clone())),
            _ => None,
        })
        .collect()
}

fn ownership(resource: &ast::Resource<String>) -> BTreeMap<String, String> {
    let mut group = String::new();
    let mut owners = BTreeMap::new();
    for entry in &resource.body {
        match entry {
            ast::Entry::GroupComment(comment) => group = comment.content.join("\n"),
            ast::Entry::Message(message) => {
                owners.insert(message.id.name.clone(), group.clone());
            },
            ast::Entry::Term(term) => {
                owners.insert(format!("-{}", term.id.name), group.clone());
            },
            _ => {},
        }
    }
    owners
}

fn headings(resource: &ast::Resource<String>) -> Vec<Vec<String>> {
    let mut headings: Vec<_> = resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::GroupComment(comment) => Some(comment.content.clone()),
            _ => None,
        })
        .collect();
    headings.sort();
    headings
}

fn keyed_entry_count(resource: &ast::Resource<String>) -> usize {
    resource
        .body
        .iter()
        .filter(|entry| matches!(entry, ast::Entry::Message(_) | ast::Entry::Term(_)))
        .count()
}

fn append_entry(source: &mut String, index: usize, group: usize, term: bool, label: bool) {
    let prefix = if term { "-" } else { "" };
    let suffix = if label { "_label" } else { "" };
    writeln!(source, "# Note {index}").unwrap();
    writeln!(
        source,
        "{prefix}group{group}_value{index}{suffix} = Value {index}"
    )
    .unwrap();
    writeln!(source, "    .tooltip = Detail {index}\n").unwrap();
}

#[test]
fn colliding_headers_keep_separate_groups_and_stable_ownership() {
    for source in [
        "## AB\nab_x = X\n\n## A-B\n",
        "## AB\nab_x = X\n\n## A-B\nab_y = Y\n",
        "## ABVariants\nab_x = X\n\n## A-BVariants\n",
    ] {
        let original = parse(source.to_owned());
        let first = sort_ftl_resource(&original);
        let formatted = parse(first.clone());
        assert_eq!(headings(&formatted), headings(&original));
        assert_eq!(payloads(&formatted), payloads(&original));
        assert_eq!(keyed_entry_count(&formatted), keyed_entry_count(&original));
        assert_eq!(ownership(&formatted), ownership(&original));
        assert_eq!(sort_ftl_resource(&formatted), first);
    }
}

#[derive(Debug)]
struct AnnotatedPattern {
    pattern: ast::Pattern<String>,
    variables: HashSet<String>,
}

fn variable(name: String) -> ast::InlineExpression<String> {
    ast::InlineExpression::VariableReference {
        id: ast::Identifier { name },
    }
}

fn pattern_strategy() -> impl Strategy<Value = AnnotatedPattern> {
    prop::collection::vec((0u8..12, 0usize..4, any::<bool>()), 1..7).prop_map(|leaves| {
        let mut variables = HashSet::new();
        let mut elements = vec![ast::PatternElement::TextElement {
            value: "Text ".to_owned(),
        }];
        for (number, depth, function) in leaves {
            let name = format!("var{number}");
            variables.insert(name.clone());
            let mut inline = variable(name);
            if function {
                inline = ast::InlineExpression::FunctionReference {
                    id: ast::Identifier {
                        name: "FUNC".to_owned(),
                    },
                    arguments: ast::CallArguments {
                        positional: vec![inline],
                        named: vec![ast::NamedArgument {
                            name: ast::Identifier {
                                name: "style".to_owned(),
                            },
                            value: ast::InlineExpression::StringLiteral {
                                value: "short".to_owned(),
                            },
                        }],
                    },
                };
            }
            for _ in 0..depth {
                inline = ast::InlineExpression::Placeable {
                    expression: Box::new(ast::Expression::Inline(inline)),
                };
            }
            elements.push(ast::PatternElement::Placeable {
                expression: ast::Expression::Inline(inline),
            });
        }
        AnnotatedPattern {
            pattern: ast::Pattern { elements },
            variables,
        }
    })
}

fn message_strategy() -> impl Strategy<Value = (ast::Message<String>, HashSet<String>)> {
    (
        prop::option::of(pattern_strategy()),
        prop::collection::vec(pattern_strategy(), 1..4),
        0u8..12,
        pattern_strategy(),
        pattern_strategy(),
    )
        .prop_map(|(value, attributes, selector, first, default)| {
            let mut expected = HashSet::new();
            let mut value = value.map(|annotated| {
                expected.extend(annotated.variables);
                annotated.pattern
            });
            let selector = format!("selector{selector}");
            expected.insert(selector.clone());
            expected.extend(first.variables);
            expected.extend(default.variables);
            let select = ast::PatternElement::Placeable {
                expression: ast::Expression::Select {
                    selector: variable(selector),
                    variants: vec![
                        ast::Variant {
                            key: ast::VariantKey::Identifier {
                                name: "one".to_owned(),
                            },
                            value: first.pattern,
                            default: false,
                        },
                        ast::Variant {
                            key: ast::VariantKey::Identifier {
                                name: "other".to_owned(),
                            },
                            value: default.pattern,
                            default: true,
                        },
                    ],
                },
            };
            let mut attributes: Vec<_> = attributes
                .into_iter()
                .enumerate()
                .map(|(index, annotated)| {
                    expected.extend(annotated.variables);
                    ast::Attribute {
                        id: ast::Identifier {
                            name: format!("attr{index}"),
                        },
                        value: annotated.pattern,
                    }
                })
                .collect();
            // Exercise attribute-only messages as well as value patterns. Each
            // select retains exactly one default variant throughout shrinking.
            if let Some(value) = &mut value {
                value.elements.push(select);
            } else {
                attributes[0].value.elements.push(select);
            }
            (
                ast::Message {
                    id: ast::Identifier {
                        name: "generated".to_owned(),
                    },
                    value,
                    attributes,
                    comment: None,
                },
                expected,
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn formatting_preserves_payloads_and_reaches_a_fixed_point(
        group_count in 1usize..9,
        entries in prop::collection::vec((0usize..8, any::<bool>(), any::<bool>()), 1..65),
        variants in any::<bool>(),
        reverse in any::<bool>(),
    ) {
        let mut source = "### Generated resource\n\n".to_owned();
        let mut groups: Vec<_> = (0..group_count).collect();
        if reverse {
            groups.reverse();
        }
        for group in groups {
            let suffix = if variants { "Variants" } else { "" };
            writeln!(source, "## Group{group}{suffix}\n").unwrap();
            for (index, &(owner, term, label)) in entries.iter().enumerate().rev() {
                if owner % group_count == group {
                    append_entry(&mut source, index, group, term, label);
                }
            }
        }
        let original = parse(source);
        let first = sort_ftl_resource(&original);
        let formatted = parse(first.clone());
        prop_assert_eq!(payloads(&formatted), payloads(&original));
        prop_assert_eq!(keyed_entry_count(&formatted), entries.len());
        prop_assert_eq!(headings(&formatted), headings(&original));
        prop_assert_eq!(sort_ftl_resource(&formatted), first);
    }

    #[test]
    fn ambiguous_headings_preserve_ownership_and_format_stably(
        group in 0usize..32,
        entries in prop::collection::vec((any::<bool>(), any::<bool>(), any::<bool>()), 1..33),
        variants in any::<bool>(),
        reverse in any::<bool>(),
    ) {
        let suffix = if variants { "Variants" } else { "" };
        let mut groups = vec![(false, format!("Group{group}{suffix}")), (true, format!("G-roup{group}{suffix}"))];
        if reverse {
            groups.reverse();
        }
        let mut source = String::new();
        for (owner, heading) in groups {
            writeln!(source, "## {heading}\n").unwrap();
            for (index, &(entry_owner, term, label)) in entries.iter().enumerate() {
                if entry_owner == owner {
                    append_entry(&mut source, index, group, term, label);
                }
            }
        }
        let original = parse(source);
        let first = sort_ftl_resource(&original);
        let formatted = parse(first.clone());
        prop_assert_eq!(headings(&formatted), headings(&original));
        prop_assert_eq!(payloads(&formatted), payloads(&original));
        prop_assert_eq!(keyed_entry_count(&formatted), entries.len());
        prop_assert_eq!(ownership(&formatted), ownership(&original));
        prop_assert_eq!(sort_ftl_resource(&formatted), first);
    }

    #[test]
    fn variable_traversal_matches_annotations((message, expected) in message_strategy()) {
        let resource = ast::Resource { body: vec![ast::Entry::Message(message)] };
        let reparsed = parse(serializer::serialize(&resource));
        let ast::Entry::Message(message) = &reparsed.body[0] else {
            panic!("generated resource must contain its message");
        };
        prop_assert_eq!(extract_variables_from_message(message), expected);
    }
}
