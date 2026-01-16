use es_fluent_derive_core::meta::TypeKind;
use es_fluent_derive_core::namer::FluentKey;
use es_fluent_derive_core::registry::{FtlTypeInfo, FtlVariant};
use es_fluent_generate::{FluentParseMode, generate};
use proc_macro2::Span;
use std::fs;
use syn::Ident;
use tempfile::TempDir;

macro_rules! static_str {
    ($s:expr) => {
        $s.to_string().leak()
    };
}

macro_rules! static_slice {
    ($($item:expr),* $(,)?) => {
        vec![$($item),*].leak() as &'static [_]
    };
}

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
    let gender_variants: &'static [FtlVariant] = static_slice![
        FtlVariant {
            name: static_str!("Female"),
            ftl_key: static_str!(
                FluentKey::from(&Ident::new("Gender", Span::call_site()))
                    .join("Female")
                    .to_string()
            ),
            args: static_slice![],
            module_path: "test",
            line: 0,
        },
        FtlVariant {
            name: static_str!("Helicopter"),
            ftl_key: static_str!(
                FluentKey::from(&Ident::new("Gender", Span::call_site()))
                    .join("Helicopter")
                    .to_string()
            ),
            args: static_slice![],
            module_path: "test",
            line: 0,
        },
        FtlVariant {
            name: static_str!("Male"),
            ftl_key: static_str!(
                FluentKey::from(&Ident::new("Gender", Span::call_site()))
                    .join("Male")
                    .to_string()
            ),
            args: static_slice![],
            module_path: "test",
            line: 0,
        },
        FtlVariant {
            name: static_str!("Other"),
            ftl_key: static_str!(
                FluentKey::from(&Ident::new("Gender", Span::call_site()))
                    .join("Other")
                    .to_string()
            ),
            args: static_slice![],
            module_path: "test",
            line: 0,
        },
    ];
    let gender = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "Gender",
        variants: gender_variants,
        file_path: "",
        module_path: "test",
    };

    // 2. HelloUser (Complete)
    let hello_user_variants: &'static [FtlVariant] = static_slice![FtlVariant {
        name: static_str!("hello_user"),
        ftl_key: static_str!(
            FluentKey::from(&Ident::new("HelloUser", Span::call_site()))
                .join("hello_user")
                .to_string()
        ),
        args: static_slice!["f0"],
        module_path: "test",
        line: 0,
    }];
    let hello_user = FtlTypeInfo {
        type_kind: TypeKind::Struct, // Assuming struct for single message
        type_name: "HelloUser",
        variants: hello_user_variants,
        file_path: "",
        module_path: "test",
    };

    // 3. Shared (Adding 'Videos' new key)
    let shared_variants: &'static [FtlVariant] = static_slice![
        FtlVariant {
            name: static_str!("Photos"),
            ftl_key: static_str!(
                FluentKey::from(&Ident::new("Shared", Span::call_site()))
                    .join("Photos")
                    .to_string()
            ),
            args: static_slice!["user_name", "photo_count", "user_gender"],
            module_path: "test",
            line: 0,
        },
        // NEW KEY
        FtlVariant {
            name: static_str!("Videos"),
            ftl_key: static_str!(
                FluentKey::from(&Ident::new("Shared", Span::call_site()))
                    .join("Videos")
                    .to_string()
            ),
            args: static_slice![],
            module_path: "test",
            line: 0,
        },
    ];
    let shared = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "Shared",
        variants: shared_variants,
        file_path: "",
        module_path: "test",
    };

    // 4. What (Complete)
    let what_variants: &'static [FtlVariant] = static_slice![FtlVariant {
        name: static_str!("Hi"),
        ftl_key: static_str!(
            FluentKey::from(&Ident::new("What", Span::call_site()))
                .join("Hi")
                .to_string()
        ),
        args: static_slice![],
        module_path: "test",
        line: 0,
    }];
    let what = FtlTypeInfo {
        type_kind: TypeKind::Enum,
        type_name: "What",
        variants: what_variants,
        file_path: "",
        module_path: "test",
    };

    let items: &[FtlTypeInfo] = static_slice![gender, hello_user, shared, what];

    // Run generate in Conservative mode
    generate(
        crate_name,
        &i18n_path,
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
