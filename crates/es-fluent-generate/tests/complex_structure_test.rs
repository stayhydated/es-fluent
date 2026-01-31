mod common;

use common::{enum_type, ftl_key, leak_slice, struct_type, variant, variant_with_args};
use es_fluent_generate::{FluentParseMode, generate};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_complex_structure_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let i18n_path = temp_dir.path().join("i18n");
    let crate_name = "complex_crate";
    let ftl_file_path = i18n_path.join(format!("{}.ftl", crate_name));

    fs::create_dir_all(&i18n_path).unwrap();

    // Initial Complex Content
    let initial_content = r#"## Gender

gender-Female = Female
gender-Helicopter = Helicopter
gender-Male = Male
gender-Other = Other

## HelloUser

hello_user = Hello, { $f0 } !

## Shared

shared-Photos =
    { $user_name } { $photo_count ->
        [one] added a new photo
       *[other] added { $photo_count } new photos
    } to { $user_gender ->
        [male] his stream
        [female] her stream
       *[other] their stream
    }.

## What

what-Hi = Hi
"#;
    fs::write(&ftl_file_path, initial_content).unwrap();

    // Define items.
    // We will mimic the existing items to ensure they are "handled" (preserved).
    // And add a NEW key to "Shared".

    // 1. Gender Group (Complete)
    let gender = enum_type(
        "Gender",
        vec![
            variant("Female", &ftl_key("Gender", "Female")),
            variant("Helicopter", &ftl_key("Gender", "Helicopter")),
            variant("Male", &ftl_key("Gender", "Male")),
            variant("Other", &ftl_key("Gender", "Other")),
        ],
    );

    // 2. HelloUser (Complete)
    let hello_user = struct_type(
        "HelloUser",
        vec![variant_with_args(
            "hello_user",
            &ftl_key("HelloUser", "hello_user"),
            vec!["f0"],
        )],
    );

    // 3. Shared (Adding 'Videos' new key)
    let shared = enum_type(
        "Shared",
        vec![
            variant_with_args(
                "Photos",
                &ftl_key("Shared", "Photos"),
                vec!["user_name", "photo_count", "user_gender"],
            ),
            // NEW KEY
            variant("Videos", &ftl_key("Shared", "Videos")),
        ],
    );

    // 4. What (Complete)
    let what = enum_type("What", vec![variant("Hi", &ftl_key("What", "Hi"))]);

    let items = leak_slice(vec![gender, hello_user, shared, what]);

    // Run generate in Conservative mode
    generate(
        crate_name,
        &i18n_path,
        temp_dir.path(),
        items,
        FluentParseMode::Conservative,
        false,
    )
    .unwrap();

    let content = fs::read_to_string(&ftl_file_path).unwrap();
    println!("Generated Content:\n{}", content);

    // Verify Shared-Photos is preserved exactly (checking a distinctive part)
    assert!(
        content.contains("[one] added a new photo"),
        "Complex selector 'one' missing"
    );
    assert!(
        content.contains("*[other] added { $photo_count } new photos"),
        "Complex selector 'other' missing"
    );
    assert!(
        content.contains("[female] her stream"),
        "Nested/Second selector missing"
    );

    // Verify new key injection in Shared group
    let photos_pos = content.find("shared-Photos").expect("Photos key missing");
    let videos_pos = content.find("shared-Videos").expect("Videos key missing");
    let shared_group_pos = content.find("## Shared").expect("Shared group missing");
    let what_group_pos = content.find("## What").expect("What group missing");

    assert!(
        shared_group_pos < photos_pos,
        "Photos should be after Shared header"
    );
    assert!(
        photos_pos < what_group_pos,
        "Photos should be before What header"
    );

    // Videos should be injected in Shared group.
    // Since Photos was matched and removed from pending, Videos is the remaining variant.
    // It should be injected at the end of the group (before ## What).
    assert!(
        shared_group_pos < videos_pos,
        "Videos should be after Shared header"
    );
    assert!(
        videos_pos < what_group_pos,
        "Videos should be before What header"
    );
}
