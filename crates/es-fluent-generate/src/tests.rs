use super::*;
use es_fluent_shared::meta::TypeKind;
use es_fluent_shared::registry::{FtlTypeInfo, FtlVariant, NamespaceRule};
use fluent_syntax::{ast, parser};
use fs_err as fs;
use indexmap::IndexMap;
use std::borrow::Cow;
use std::path::PathBuf;

fn leak_str(s: impl ToString) -> &'static str {
    s.to_string().leak()
}

fn leak_slice<T>(items: Vec<T>) -> &'static [T] {
    items.leak()
}

fn test_variant(name: &str, ftl_key: &str, args: &[&str]) -> FtlVariant {
    test_variant_at(name, ftl_key, args, 0)
}

fn test_variant_at(name: &str, ftl_key: &str, args: &[&str], line: u32) -> FtlVariant {
    FtlVariant {
        name: leak_str(name),
        ftl_key: leak_str(ftl_key),
        args: leak_slice(args.iter().map(|arg| leak_str(arg)).collect()),
        module_path: "test",
        line,
    }
}

fn test_type(name: &str, variants: Vec<FtlVariant>) -> FtlTypeInfo {
    test_type_at(name, variants, "")
}

fn test_type_at(name: &str, variants: Vec<FtlVariant>, file_path: &str) -> FtlTypeInfo {
    FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: leak_str(name),
        variants: leak_slice(variants),
        file_path: leak_str(file_path),
        module_path: "test",
        namespace: None,
    }
}

fn parse_resource_allowing_errors(input: &str) -> ast::Resource<String> {
    parser::parse(input.to_string()).unwrap_or_else(|(resource, _)| resource)
}

fn owned_variant(name: &str, ftl_key: &str, args: &[&str]) -> OwnedVariant {
    OwnedVariant::new(name, ftl_key, args.iter().copied()).expect("owned variant")
}

#[test]
fn owned_type_info_and_entry_helpers_work() {
    let info = test_type(
        "Greeter",
        vec![test_variant("HelloName", "greeter-hello_name", &["name"])],
    );

    let owned = OwnedTypeInfo::from_ftl_type_info(&info).expect("owned type info");
    assert_eq!(owned.type_name, "Greeter");
    assert_eq!(owned.variants.len(), 1);
    assert_eq!(owned.variants[0].ftl_key(), "greeter-hello_name");

    let message = create_message_entry(&owned.variants[0]);
    assert!(matches!(
        &message,
        ast::Entry::Message(msg) if msg.id.name == "greeter-hello_name"
    ));

    let group = create_group_comment_entry("Greeter");
    assert!(matches!(
        &group,
        ast::Entry::GroupComment(comment)
            if group_comment_name(comment) == Some("Greeter".to_string())
    ));
}

#[test]
fn generate_rejects_duplicate_keys_within_one_type_before_writing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = temp.path().join("i18n");
    let item = test_type_at(
        "LoginError",
        vec![
            test_variant_at("MissingUser", "login_error-same", &["name"], 10),
            test_variant_at("InvalidPassword", "login_error-same", &["attempts"], 11),
        ],
        "src/login.rs",
    );

    let err = generate(
        "demo",
        &output,
        temp.path(),
        &[item],
        FluentParseMode::Conservative,
        false,
    )
    .expect_err("duplicate key should fail");

    let message = err.to_string();
    assert!(message.contains("Duplicate generated FTL key 'login_error-same'"));
    assert!(message.contains("LoginError"));
    assert!(message.contains("MissingUser"));
    assert!(message.contains("InvalidPassword"));
    assert!(message.contains("src/login.rs:10"));
    assert!(message.contains("src/login.rs:11"));
    assert!(!output.join("demo.ftl").exists());
}

#[test]
fn generate_rejects_duplicate_keys_across_types() {
    let temp = tempfile::tempdir().expect("tempdir");
    let first = test_type_at(
        "SaveButton",
        vec![test_variant_at("SaveButton", "shared-key", &[], 3)],
        "src/save.rs",
    );
    let second = test_type_at(
        "CancelButton",
        vec![test_variant_at("CancelButton", "shared-key", &[], 9)],
        "src/cancel.rs",
    );

    let err = generate(
        "demo",
        temp.path().join("i18n"),
        temp.path(),
        &[first, second],
        FluentParseMode::Aggressive,
        true,
    )
    .expect_err("duplicate key should fail");

    let message = err.to_string();
    assert!(message.contains("Duplicate generated FTL key 'shared-key'"));
    assert!(message.contains("SaveButton"));
    assert!(message.contains("CancelButton"));
    assert!(message.contains("src/save.rs:3"));
    assert!(message.contains("src/cancel.rs:9"));
}

#[test]
fn generate_rejects_label_key_colliding_with_message_key() {
    let temp = tempfile::tempdir().expect("tempdir");
    let label = test_type_at(
        "SettingsLabel",
        vec![test_variant_at(
            "SettingsLabel",
            "settings_label_label",
            &[],
            4,
        )],
        "src/settings.rs",
    );
    let message = test_type_at(
        "SettingsMessage",
        vec![test_variant_at(
            "SettingsMessage",
            "settings_label_label",
            &["name"],
            12,
        )],
        "src/message.rs",
    );

    let err = generate(
        "demo",
        temp.path().join("i18n"),
        temp.path(),
        &[label, message],
        FluentParseMode::Conservative,
        true,
    )
    .expect_err("label/message collision should fail");

    let message = err.to_string();
    assert!(message.contains("settings_label_label"));
    assert!(message.contains("SettingsLabel"));
    assert!(message.contains("SettingsMessage"));
}

#[test]
fn clean_rejects_duplicate_keys_with_different_argument_sets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = temp.path().join("i18n");
    fs::create_dir_all(&output).expect("create output");
    let file = output.join("demo.ftl");
    fs::write(&file, "duplicate-key = Existing\n").expect("write existing");

    let first = test_type_at(
        "First",
        vec![test_variant_at("First", "duplicate-key", &["name"], 1)],
        "src/first.rs",
    );
    let second = test_type_at(
        "Second",
        vec![test_variant_at("Second", "duplicate-key", &["count"], 2)],
        "src/second.rs",
    );

    let err = clean::clean("demo", &output, temp.path(), &[first, second], false)
        .expect_err("clean should fail before merging duplicate generated keys");

    let message = err.to_string();
    assert!(message.contains("Duplicate generated FTL key 'duplicate-key'"));
    assert!(message.contains("First"));
    assert!(message.contains("Second"));
    assert!(
        fs::read_to_string(&file)
            .expect("read")
            .contains("Existing")
    );
}

#[test]
fn read_existing_and_write_updated_resource_cover_io_branches() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("example.ftl");

    let missing = read_existing_resource(&file_path).expect("missing resource");
    assert!(missing.body.is_empty());

    fs::write(&file_path, "   \n").expect("write whitespace");
    let empty = read_existing_resource(&file_path).expect("empty resource");
    assert!(empty.body.is_empty());

    fs::write(&file_path, "broken = {\n").expect("write invalid");
    let err = read_existing_resource(&file_path).expect_err("invalid resource should fail");
    assert!(err.to_string().contains("Refusing to use"));

    let updated = parse_resource_allowing_errors("updated = value\n");
    let dry_changed =
        write_updated_resource(&file_path, &updated, true, formatting::sort_ftl_resource)
            .expect("dry run");
    assert!(dry_changed);
    assert!(
        fs::read_to_string(&file_path)
            .expect("read")
            .contains("broken")
    );

    let changed =
        write_updated_resource(&file_path, &updated, false, formatting::sort_ftl_resource)
            .expect("write update");
    assert!(changed);
    let unchanged =
        write_updated_resource(&file_path, &updated, false, formatting::sort_ftl_resource)
            .expect("write unchanged");
    assert!(!unchanged);

    let empty_resource = ast::Resource { body: vec![] };
    let emptied = write_updated_resource(
        &file_path,
        &empty_resource,
        false,
        formatting::sort_ftl_resource,
    )
    .expect("write empty");
    assert!(emptied);
    assert_eq!(fs::read_to_string(&file_path).expect("read empty"), "");
}

#[test]
fn write_or_preview_and_print_diff_cover_preview_and_write_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("nested/preview.ftl");

    write_or_preview(&file_path, "old = value\n", "new = value\n", false, true)
        .expect("dry-run preview");
    print_diff("old = value\n", "new = value\n");

    write_or_preview(&file_path, "", "", true, false).expect("real write");
    assert!(file_path.exists());
}

#[test]
fn write_updated_resource_covers_unchanged_empty_and_dry_run_empty_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("empty.ftl");
    fs::write(&file_path, "").expect("write empty file");

    let empty_resource = ast::Resource { body: vec![] };
    let unchanged = write_updated_resource(
        &file_path,
        &empty_resource,
        false,
        formatting::sort_ftl_resource,
    )
    .expect("unchanged empty write");
    assert!(!unchanged);

    let unchanged_dry_run = write_updated_resource(
        &file_path,
        &empty_resource,
        true,
        formatting::sort_ftl_resource,
    )
    .expect("unchanged dry run");
    assert!(!unchanged_dry_run);

    write_or_preview(&file_path, "old = value\n", "", true, true)
        .expect("dry-run empty from non-empty");
    write_or_preview(&file_path, "", "", true, true).expect("dry-run empty from empty");
}

#[test]
fn print_diff_handles_equal_lines_and_multiple_groups() {
    let old = "line1 = old\nkeep1 = 1\nkeep2 = 2\nkeep3 = 3\nkeep4 = 4\nkeep5 = 5\nkeep6 = 6\nkeep7 = 7\nkeep8 = 8\nkeep9 = 9\nkeep10 = 10\nline12 = old\n";
    let new = "line1 = new\nkeep1 = 1\nkeep2 = 2\nkeep3 = 3\nkeep4 = 4\nkeep5 = 5\nkeep6 = 6\nkeep7 = 7\nkeep8 = 8\nkeep9 = 9\nkeep10 = 10\nline12 = new\n";
    print_diff(old, new);
}

#[test]
fn collect_existing_keys_and_remove_empty_group_comments_cover_terms_and_pending_groups() {
    let resource = parse_resource_allowing_errors(
        "## Empty\n# orphan-comment\n\n## Keep\nkeep = yes\n\n-shared = shared\n",
    );

    let keys = collect_existing_keys(&resource);
    assert!(keys.contains("keep"));
    assert!(keys.contains("-shared"));

    let cleaned = remove_empty_group_comments(resource);
    let formatted = formatting::sort_ftl_resource(&cleaned);
    assert!(!formatted.contains("## Empty"));
    assert!(formatted.contains("## Keep"));
    assert!(formatted.contains("-shared = shared"));
}

#[test]
fn remove_empty_group_comments_keeps_top_level_entries_without_group() {
    let resource = parse_resource_allowing_errors("top-level = value\n# loose comment\n");
    let cleaned = remove_empty_group_comments(resource);
    let formatted = formatting::sort_ftl_resource(&cleaned);
    assert!(formatted.contains("top-level = value"));
    assert!(formatted.contains("# loose comment"));
}

#[test]
fn insert_late_relocated_handles_empty_groups_and_duplicate_names() {
    let mut no_groups = vec![create_message_entry(&owned_variant(
        "Only",
        "only-key",
        &[],
    ))];
    let mut late = IndexMap::new();
    late.insert(
        "MissingGroup".to_string(),
        vec![create_message_entry(&owned_variant(
            "Late",
            "late-key",
            &[],
        ))],
    );
    insert_late_relocated(&mut no_groups, &late);
    assert_eq!(no_groups.len(), 1);

    let mut body = parse_resource_allowing_errors(
        "## GroupA\ngroup_a-A1 = A1\n\n## GroupB\ngroup_b-B1 = B1\n\n## GroupA\ngroup_a-A2 = A2\n",
    )
    .body;
    let mut late_for_group = IndexMap::new();
    late_for_group.insert(
        "GroupA".to_string(),
        vec![create_message_entry(&owned_variant(
            "LateA",
            "group_a-late",
            &[],
        ))],
    );
    insert_late_relocated(&mut body, &late_for_group);

    let inserted_count = body
        .iter()
        .filter(|entry| matches!(entry, ast::Entry::Message(msg) if msg.id.name == "group_a-late"))
        .count();
    assert_eq!(inserted_count, 1);
}

#[test]
fn smart_merge_moves_leading_comments_with_relocated_messages_and_terms() {
    let group_a = test_type(
        "GroupA",
        vec![
            test_variant("A1", "group_a-A1", &[]),
            test_variant("Term", "-group_a-term", &[]),
        ],
    );
    let group_b = test_type("GroupB", vec![test_variant("B1", "group_b-B1", &[])]);
    let items = vec![&group_a, &group_b];

    let existing = parse_resource_allowing_errors(
        "## GroupA\n# move-with-message\ngroup_b-B1 = wrong-group\n\n## GroupB\n# move-with-term\n-group_a-term = wrong-group\n",
    );
    let merged = smart_merge(existing, &items, MergeBehavior::Append).expect("merge");
    let content = fluent_syntax::serializer::serialize(&merged);

    let group_a_pos = content.find("## GroupA").expect("group a");
    let group_b_pos = content.find("## GroupB").expect("group b");
    let message_comment_pos = content
        .find("# move-with-message")
        .expect("message comment");
    let message_pos = content.find("group_b-B1 = wrong-group").expect("message");
    let term_comment_pos = content.find("# move-with-term").expect("term comment");
    let term_pos = content.find("-group_a-term = wrong-group").expect("term");

    assert!(message_comment_pos > group_b_pos);
    assert!(message_comment_pos < message_pos);
    assert!(term_comment_pos > group_a_pos);
    assert!(term_comment_pos < term_pos);
}

#[test]
fn smart_merge_covers_relocation_terms_junk_and_cleanup_modes() {
    let group_a = test_type("GroupA", vec![test_variant("A1", "group_a-A1", &[])]);
    let group_b = test_type(
        "GroupB",
        vec![
            test_variant("B1", "group_b-B1", &[]),
            test_variant("SharedTerm", "-shared_term", &[]),
        ],
    );
    let items = vec![&group_a, &group_b];

    let existing_append = parse_resource_allowing_errors(
        "## GroupA\ngroup_b-B1 = wrong-group\n\n## GroupB\n-shared_term = shared\nbroken = {\n",
    );
    let merged_append =
        smart_merge(existing_append, &items, MergeBehavior::Append).expect("append merge");
    let merged_append_text = formatting::sort_ftl_resource(&merged_append);
    assert!(merged_append_text.contains("## GroupA"));
    assert!(merged_append_text.contains("## GroupB"));
    assert!(merged_append_text.contains("group_b-B1 = wrong-group"));
    assert!(merged_append_text.contains("-shared_term = shared"));

    let existing_clean = parse_resource_allowing_errors(
        "## GroupA\ngroup_b-B1 = wrong-group\n\n## GroupB\n-shared_term = shared\nbroken = {\n",
    );
    let merged_clean =
        smart_merge(existing_clean, &items, MergeBehavior::Clean).expect("clean merge");
    let merged_clean_text = formatting::sort_ftl_resource(&merged_clean);
    assert!(merged_clean_text.contains("-shared_term = shared"));
    assert!(merged_clean_text.contains("group_b-B1 = wrong-group"));
    assert!(!merged_clean_text.contains("group_a-A1"));
}

#[test]
fn smart_merge_handles_duplicates_empty_group_headers_and_comment_entries() {
    let group_a = test_type(
        "GroupA",
        vec![
            test_variant("A1", "dup-key", &[]),
            test_variant("SharedTerm", "-dup-term", &[]),
        ],
    );
    let items = vec![&group_a];

    let mut existing = parse_resource_allowing_errors(
        "## GroupA\ndup-key = first\ndup-key = second\n-dup-term = one\n-dup-term = two\n",
    );
    existing.body.push(ast::Entry::Comment(ast::Comment {
        content: vec!["loose-comment".to_string()],
    }));
    existing
        .body
        .push(ast::Entry::GroupComment(ast::Comment { content: vec![] }));

    let merged = smart_merge(existing, &items, MergeBehavior::Append).expect("merge");
    let merged_text = formatting::sort_ftl_resource(&merged);
    assert_eq!(merged_text.matches("dup-key =").count(), 1);
    assert_eq!(merged_text.matches("-dup-term =").count(), 1);
    assert!(merged_text.contains("# loose-comment"));
}

#[test]
fn smart_merge_appends_relocated_entries_for_group_switch_and_missing_group_header() {
    let group_x = test_type("GroupX", vec![]);
    let group_a = test_type(
        "GroupA",
        vec![
            test_variant("A1", "group_a-A1", &[]),
            test_variant("A2", "group_a-A2", &[]),
        ],
    );
    let group_b = test_type("GroupB", vec![test_variant("B1", "group_b-B1", &[])]);
    let group_c = test_type("GroupC", vec![test_variant("C1", "group_c-C1", &[])]);
    let items = vec![&group_x, &group_a, &group_b, &group_c];

    let existing = parse_resource_allowing_errors(
        "## GroupX\ngroup_a-A1 = moved-to-a\ngroup_b-B1 = moved-to-b\n\n## GroupA\ngroup_a-A2 = keep-a2\n\n## GroupC\ngroup_c-C1 = keep-c1\n",
    );
    let merged = smart_merge(existing, &items, MergeBehavior::Append).expect("merge");
    let merged_text = formatting::sort_ftl_resource(&merged);

    assert!(merged_text.contains("group_a-A1 = moved-to-a"));
    assert!(merged_text.contains("## GroupB"));
    assert!(merged_text.contains("group_b-B1 = moved-to-b"));
}

#[test]
fn generate_creates_namespaced_directories_and_handles_dry_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let i18n_root = temp.path().join("i18n");

    let mut namespaced = test_type("NamespacedType", vec![test_variant("A1", "ns-a1", &[])]);
    namespaced.namespace = Some(es_fluent_shared::registry::NamespaceRule::Literal(
        std::borrow::Cow::Borrowed("ui"),
    ));
    let items = vec![&namespaced];

    let changed = generate(
        "crate-name",
        &i18n_root,
        temp.path(),
        &items,
        FluentParseMode::Conservative,
        false,
    )
    .expect("generate namespaced");
    assert!(changed);
    assert!(i18n_root.join("crate-name/ui.ftl").exists());

    let dry_run_path = PathBuf::from("dry_run/absent.ftl");
    write_or_preview(&dry_run_path, "a = b\n", "a = c\n", false, true).expect("dry run");
}

#[test]
fn plan_outputs_uses_canonical_resource_specs_for_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let i18n_root = temp.path().join("i18n");

    let base = test_type("BaseType", vec![test_variant("Base", "base", &[])]);
    let mut namespaced = test_type("NamespacedType", vec![test_variant("A1", "ns-a1", &[])]);
    namespaced.namespace = Some(NamespaceRule::Literal(Cow::Borrowed("ui/forms")));
    let items = vec![&base, &namespaced];

    let outputs = crate::pipeline::plan_outputs("crate-name", &i18n_root, temp.path(), &items)
        .expect("planned outputs");
    let base_output = outputs
        .iter()
        .find(|output| output.resource.key.as_str() == "crate-name")
        .expect("base output");
    let namespace_output = outputs
        .iter()
        .find(|output| output.resource.key.as_str() == "crate-name/ui/forms")
        .expect("namespaced output");

    assert_eq!(base_output.resource.locale_relative_path, "crate-name.ftl");
    assert_eq!(
        namespace_output.resource.locale_relative_path,
        "crate-name/ui/forms.ftl"
    );
    assert_eq!(base_output.file_path, i18n_root.join("crate-name.ftl"));
    assert_eq!(
        namespace_output.file_path,
        i18n_root.join("crate-name/ui/forms.ftl")
    );
}

#[test]
fn generate_rejects_namespace_paths_that_escape_the_crate_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let i18n_root = temp.path().join("i18n");

    let escaping = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "EscapingType",
        variants: leak_slice(vec![test_variant("Hello", "hello", &[])]),
        file_path: "src/../escape.rs",
        module_path: "test",
        namespace: Some(NamespaceRule::FileRelative),
    };

    let literal_escape = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "LiteralEscape",
        variants: leak_slice(vec![test_variant("Bye", "bye", &[])]),
        file_path: "src/lib.rs",
        module_path: "test",
        namespace: Some(NamespaceRule::Literal(Cow::Borrowed("../literal-escape"))),
    };

    let err = generate(
        "crate-name",
        &i18n_root,
        temp.path(),
        &[&escaping, &literal_escape],
        FluentParseMode::Conservative,
        true,
    )
    .expect_err("escaping namespace should be rejected");

    assert!(
        err.to_string().contains("Invalid namespace '../escape'")
            || err
                .to_string()
                .contains("Invalid namespace '../literal-escape'")
    );
    assert!(
        !i18n_root.join("escape.ftl").exists(),
        "generation should not create escaped output"
    );
}

#[test]
fn generate_rejects_noncanonical_namespace_literals() {
    let temp = tempfile::tempdir().expect("tempdir");
    let i18n_root = temp.path().join("i18n");

    let padded = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "PaddedNamespace",
        variants: leak_slice(vec![test_variant("Hello", "hello", &[])]),
        file_path: "src/lib.rs",
        module_path: "test",
        namespace: Some(NamespaceRule::Literal(Cow::Borrowed(" ui "))),
    };

    let with_extension = FtlTypeInfo {
        type_kind: TypeKind::Struct,
        type_name: "FileNamespace",
        variants: leak_slice(vec![test_variant("Bye", "bye", &[])]),
        file_path: "src/lib.rs",
        module_path: "test",
        namespace: Some(NamespaceRule::Literal(Cow::Borrowed("ui.ftl"))),
    };

    let err = generate(
        "crate-name",
        &i18n_root,
        temp.path(),
        &[&padded, &with_extension],
        FluentParseMode::Conservative,
        true,
    )
    .expect_err("noncanonical namespaces should be rejected");

    let error_text = err.to_string();
    assert!(
        error_text.contains("Invalid namespace ' ui '")
            || error_text.contains("Invalid namespace 'ui.ftl'")
    );
}

#[test]
fn generate_rejects_invalid_registered_keys_and_arguments() {
    let temp = tempfile::tempdir().expect("tempdir");
    let i18n_root = temp.path().join("i18n");

    let invalid_key = test_type(
        "InvalidKey",
        vec![test_variant("Broken", "_invalid", &["name"])],
    );
    let key_err = generate(
        "crate-name",
        &i18n_root,
        temp.path(),
        &[&invalid_key],
        FluentParseMode::Conservative,
        true,
    )
    .expect_err("invalid key should fail");
    assert!(
        key_err
            .to_string()
            .contains("Invalid Fluent metadata '_invalid'")
    );

    let invalid_arg = test_type(
        "InvalidArg",
        vec![test_variant("Broken", "valid-key", &["not valid"])],
    );
    let arg_err = generate(
        "crate-name",
        &i18n_root,
        temp.path(),
        &[&invalid_arg],
        FluentParseMode::Conservative,
        true,
    )
    .expect_err("invalid argument should fail");
    assert!(
        arg_err
            .to_string()
            .contains("Invalid Fluent metadata 'not valid'")
    );
}
