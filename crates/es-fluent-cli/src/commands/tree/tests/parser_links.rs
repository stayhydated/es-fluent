use super::*;

#[test]
fn tree_args_show_attributes_and_variables_by_default() {
    let default = TreeArgs::try_parse_from(["tree"]).expect("default tree args parse");
    assert!(default.attributes);
    assert!(default.variables);
    assert_eq!(default.link_mode, None);

    let hidden = TreeArgs::try_parse_from(["tree", "--no-attributes", "--no-variables"])
        .expect("negative detail flags parse");
    assert!(!hidden.attributes);
    assert!(!hidden.variables);

    let ftl_links =
        TreeArgs::try_parse_from(["tree", "--link-mode", "ftl"]).expect("ftl link mode parses");
    assert_eq!(ftl_links.link_mode.as_deref(), Some("ftl"));

    assert!(TreeArgs::try_parse_from(["tree", "--attributes"]).is_err());
    assert!(TreeArgs::try_parse_from(["tree", "--variables"]).is_err());
}

#[test]
fn tree_link_mode_parse_arg_rejects_invalid_values() {
    assert_eq!(TreeLinkMode::parse_arg("rust").unwrap(), TreeLinkMode::Rust);
    assert_eq!(TreeLinkMode::parse_arg("ftl").unwrap(), TreeLinkMode::Ftl);

    let error = TreeLinkMode::parse_arg("bad").expect_err("bad mode should fail");
    assert!(error.to_string().contains("invalid link mode 'bad'"));
}

#[test]
fn ftl_source_map_finds_entry_attribute_and_variable_positions() {
    let content = "greeting = Hello { $name }\n    .title = Title for { $name }\ncount = { $num ->\n    [one] One\n   *[other] { $num }\n}\n-term = Term { $value }\n";
    let source_map = FtlSourceMap::new(content);

    let greeting = source_map.find_message("greeting").unwrap();
    assert_eq!(greeting.id_position, position(1, 1));
    assert_eq!(
        source_map.find_attribute(greeting, "title"),
        Some(position(2, 5))
    );
    assert_eq!(
        source_map.find_variable(greeting, "name"),
        Some(position(1, 20))
    );

    let count = source_map.find_message("count").unwrap();
    assert_eq!(count.id_position, position(3, 1));
    assert_eq!(
        source_map.find_variable(count, "num"),
        Some(position(3, 11))
    );

    let term = source_map.find_term("term").unwrap();
    assert_eq!(term.id_position, position(7, 1));
    assert_eq!(
        source_map.find_variable(term, "value"),
        Some(position(7, 16))
    );
}

#[test]
fn test_extract_variables_simple() {
    let content = "hello = Hello { $name }!";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "hello").unwrap();

    let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
        msg.value.as_ref(),
        &msg.attributes,
    )
    .into_iter()
    .collect();
    variables.sort();

    assert_eq!(variables, vec!["name"]);
}

#[test]
fn test_extract_variables_multiple() {
    let content = "greeting = Hello { $name }, you have { $count } messages";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "greeting").unwrap();

    let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
        msg.value.as_ref(),
        &msg.attributes,
    )
    .into_iter()
    .collect();
    variables.sort();

    assert_eq!(variables, vec!["count", "name"]);
}

#[test]
fn test_extract_variables_select() {
    let content = "count = { $num ->\n    [one] One item\n       *[other] { $num } items\n    }";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "count").unwrap();

    let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
        msg.value.as_ref(),
        &msg.attributes,
    )
    .into_iter()
    .collect();
    variables.sort();

    assert_eq!(variables, vec!["num"]);
}

#[test]
fn test_extract_variables_nested() {
    let content = r#"message = Hello { $user }, today is { DATETIME($date) }"#;
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "message").unwrap();

    let mut variables: Vec<_> = crate::ftl::extract_variables_from_value_and_attributes(
        msg.value.as_ref(),
        &msg.attributes,
    )
    .into_iter()
    .collect();
    variables.sort();

    assert_eq!(variables, vec!["date", "user"]);
}

#[test]
fn test_build_message_tree_simple() {
    let content = "hello = Hello World";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "hello").unwrap();

    let tree = renderer(false, false).build_message_tree("hello", msg);

    match tree {
        Tree::Leaf(lines) => assert_eq!(lines, vec!["hello"]),
        _ => panic!("Expected leaf node"),
    }
}

#[test]
fn test_build_message_tree_with_attributes() {
    let content = "button = Button\n    .tooltip = Click me\n    .aria-label = Submit";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "button").unwrap();

    let tree = renderer(true, false).build_message_tree("button", msg);

    match tree {
        Tree::Node(label, children) => {
            assert_eq!(label, "button");
            assert_eq!(children.len(), 2);
        },
        _ => panic!("Expected node with children"),
    }
}

#[test]
fn test_build_message_tree_with_variables() {
    let content = "greeting = Hello { $name }";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "greeting").unwrap();

    let tree = renderer(false, true).build_message_tree("greeting", msg);

    match tree {
        Tree::Node(label, children) => {
            assert_eq!(label, "greeting");
            assert_eq!(children.len(), 1);
        },
        _ => panic!("Expected node with children"),
    }
}

#[test]
fn test_build_entry_children_no_attributes_no_variables() {
    let children = renderer(false, false).build_entry_children(&[], None);
    assert!(children.is_empty());
}

#[test]
fn test_build_entry_children_attributes_only() {
    let content = "button = Button\n    .tooltip = Click me";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "button").unwrap();

    let children = renderer(true, false).build_entry_children(&msg.attributes, msg.value.as_ref());

    assert_eq!(children.len(), 1);
}

#[test]
fn hidden_attributes_do_not_contribute_visible_variables() {
    let content = "button = Button { $label }\n    .tooltip = Tooltip { $tooltip }";
    let resource = parse_ftl(content);
    let msg = get_message(&resource, "button").unwrap();

    let children = renderer(false, true).build_entry_children(&msg.attributes, msg.value.as_ref());

    assert_eq!(children.len(), 1);
    let output = children[0].render_to_string();
    assert!(output.contains("$label"));
    assert!(!output.contains("$tooltip"));
}

#[test]
fn test_build_file_tree_nonexistent() {
    let error = renderer(false, false)
        .build_file_tree("test.ftl", Path::new("/nonexistent/path.ftl"))
        .expect_err("missing file should fail");

    assert!(
        error
            .to_string()
            .contains("failed to read FTL file 'test.ftl'")
    );
}

#[test]
#[serial_test::serial(process)]
fn build_file_tree_adds_terminal_links_for_entries_and_variables() {
    temp_env::with_var("FORCE_HYPERLINK", Some("1"), || {
        let temp = tempfile::tempdir().expect("tempdir");
        let ftl_path = temp.path().join("test-app.ftl");
        fs::write(&ftl_path, "greeting = Hello { $name }\n").expect("write ftl");

        let tree = renderer(false, true)
            .build_file_tree("test-app.ftl", &ftl_path)
            .expect("build file tree");
        let output = tree.render_to_string();

        assert!(output.contains("\u{1b}]8;;file://"));
        assert!(output.contains(&format!("file://{}", ftl_path.display())));
        assert!(output.contains(&format!("file://{}:1:1", ftl_path.display())));
        assert!(output.contains(&format!("file://{}:1:20", ftl_path.display())));
    });
}

#[test]
fn build_file_tree_link_mode_selects_rust_or_ftl_targets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ftl_path = temp.path().join("test-app.ftl");
    let rust_path = temp.path().join("src/lib.rs");
    fs::create_dir_all(rust_path.parent().unwrap()).expect("create src dir");
    fs::write(&ftl_path, "greeting = Hello { $name }\n").expect("write ftl");
    fs::write(&rust_path, "pub struct Greeting;\n").expect("write rust");

    let rust_links = RustLinkIndex::from_inventory(
        temp.path(),
        es_fluent_runner::InventoryData {
            expected_keys: vec![es_fluent_runner::ExpectedKey {
                key: es_fluent_shared::fluent::FluentMessageKey::new(
                    es_fluent_shared::fluent::FluentDomain::try_new("test-app").expect("owner"),
                    es_fluent_shared::fluent::FluentDomain::try_new("test-app").expect("domain"),
                    es_fluent_shared::fluent::FluentEntryId::try_new("greeting").expect("key"),
                ),
                variables: vec![
                    es_fluent_shared::fluent::FluentArgumentName::try_new("name")
                        .expect("variable"),
                ],
                resource: Some(es_fluent_shared::resource::ModuleResourceSpec::base(
                    "test-app", true,
                )),
                source_file: es_fluent_shared::source::SourceFile::new("src/lib.rs"),
                source_line: Some(es_fluent_shared::source::SourceLine::new(42)),
            }],
        },
    );

    let rust_renderer = TreeRenderer::new(false, true, true, TreeLinkMode::Rust, Some(&rust_links));
    let rust_output = rust_renderer
        .build_file_tree("test-app.ftl", &ftl_path)
        .expect("build Rust-linked tree")
        .render_to_string();

    assert!(rust_output.contains(&format!("file://{}:42:1", rust_path.display())));
    assert!(!rust_output.contains(&format!("file://{}:1:1", ftl_path.display())));
    assert!(!rust_output.contains(&format!("file://{}:1:20", ftl_path.display())));

    let ftl_renderer = TreeRenderer::new(false, true, true, TreeLinkMode::Ftl, Some(&rust_links));
    let ftl_output = ftl_renderer
        .build_file_tree("test-app.ftl", &ftl_path)
        .expect("build FTL-linked tree")
        .render_to_string();

    assert!(ftl_output.contains(&format!("file://{}:1:1", ftl_path.display())));
    assert!(ftl_output.contains(&format!("file://{}:1:20", ftl_path.display())));
    assert!(!ftl_output.contains(&format!("file://{}:42:1", rust_path.display())));
}

#[test]
fn test_tree_render_basic() {
    let tree = Tree::Node(
        "root".to_string(),
        vec![
            Tree::Leaf(vec!["item1".to_string()]),
            Tree::Leaf(vec!["item2".to_string()]),
        ],
    );

    let output = tree.render_to_string();
    assert!(output.contains("root"));
    assert!(output.contains("item1"));
    assert!(output.contains("item2"));
}

#[test]
fn test_tree_render_nested() {
    let tree = Tree::Node(
        "crate".to_string(),
        vec![Tree::Node(
            "en".to_string(),
            vec![Tree::Leaf(vec!["message".to_string()])],
        )],
    );

    let output = tree.render_to_string();
    assert!(output.contains("crate"));
    assert!(output.contains("en"));
    assert!(output.contains("message"));
}

#[test]
fn test_build_term_tree_and_print_crate_tree() {
    let temp = create_workspace_with_tree_data();
    let krate = crate_info_from_temp(&temp);

    // Exercise print path for crate tree.
    let printed = print_crate_tree(&krate, false, true, true, false, TreeLinkMode::Rust, None);
    assert!(printed.is_ok());

    let resource = parse_ftl("-term = Term\n");
    let term = resource
        .body
        .iter()
        .find_map(|entry| match entry {
            ast::Entry::Term(term) => Some(term),
            _ => None,
        })
        .expect("term exists");
    let tree = renderer(false, false).build_term_tree(&term.id.name, term);
    match tree {
        Tree::Leaf(lines) => assert!(lines[0].contains("-term")),
        _ => panic!("expected leaf term tree"),
    }
}
