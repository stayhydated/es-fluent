use darling::FromDeriveInput;
use es_fluent_core::options::{
    r#enum::{EnumChoiceOpts, EnumOpts},
    r#struct::{StructKvOpts, StructOpts},
};
use es_fluent_core::strategy::DisplayStrategy;
use syn::{DeriveInput, parse_quote};

#[test]
fn enum_display_strategy_default_is_fluent() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent] // no explicit display -> default for enums is FluentDisplay
        enum MyEnum {
            Unit,
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    assert_eq!(DisplayStrategy::from(&opts), DisplayStrategy::FluentDisplay);
    assert!(!opts.attr_args().is_this());
    assert!(!opts.attr_args().is_choice());
}

#[test]
fn enum_display_strategy_override_std_and_this() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(display = "std", this)]
        enum MyEnum {
            Unit,
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    assert_eq!(DisplayStrategy::from(&opts), DisplayStrategy::StdDisplay);
    assert!(opts.attr_args().is_this());
}

#[test]
fn enum_variants_and_fields_skipping_and_choice() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent]
        enum MyEnum {
            // Struct variant with one skipped field and one choice field
            Data {
                #[fluent(choice)]
                a: i32,
                #[fluent(skip)]
                b: String,
            },
            // Tuple variant with one skipped positional field
            Tuple(#[fluent(skip)] i32, String),
            // Unit variant
            Unit,
            // Entirely skipped variant (should still be parsed, filtering is done at analysis stage)
            #[fluent(skip)]
            Skipped,
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let variants = opts.variants();

    // Find Data variant
    let data = variants
        .iter()
        .find(|v| v.ident().to_string() == "Data")
        .expect("Data variant present");
    assert!(matches!(data.style(), darling::ast::Style::Struct));
    // fields() filters out skipped fields
    let data_fields = data.fields();
    assert_eq!(data_fields.len(), 1, "Only non-skipped field remains");
    assert_eq!(
        data_fields[0].ident().as_ref().unwrap().to_string(),
        "a",
        "Expected remaining field to be 'a'"
    );
    assert!(
        data_fields[0].is_choice(),
        "Field 'a' should be marked as choice"
    );

    // Tuple variant: one skipped, one retained
    let tuple = variants
        .iter()
        .find(|v| v.ident().to_string() == "Tuple")
        .expect("Tuple variant present");
    assert!(matches!(tuple.style(), darling::ast::Style::Tuple));
    assert_eq!(tuple.all_fields().len(), 2, "All tuple fields parsed");
    assert_eq!(tuple.fields().len(), 1, "One tuple field was skipped");
    // For tuple fields, idents are None, so just ensure the remaining one is present
    assert!(tuple.fields()[0].ident().is_none());

    // Unit variant style check
    let unit = variants
        .iter()
        .find(|v| v.ident().to_string() == "Unit")
        .expect("Unit variant present");
    assert!(matches!(unit.style(), darling::ast::Style::Unit));
    assert!(unit.fields().is_empty());
}

#[test]
fn struct_display_strategy_default_is_fluent() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent] // no explicit display -> default for structs is FluentDisplay
        struct MyStruct {
            a: i32,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(DisplayStrategy::from(&opts), DisplayStrategy::FluentDisplay);
    assert!(!opts.attr_args().is_this());
}

#[test]
fn struct_display_strategy_override_fluent_and_this() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(display = "fluent", this)]
        struct MyStruct {
            a: i32,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(DisplayStrategy::from(&opts), DisplayStrategy::FluentDisplay);
    assert!(opts.attr_args().is_this());
}

#[test]
fn struct_kv_keys_parsing_and_field_skipping() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["Error", "Notice"], this)]
        struct MyStruct {
            a: i32,
            #[fluent_kv(skip)]
            b: String,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");

    // ftl_enum_ident is <StructName>Ftl
    assert_eq!(opts.ftl_enum_ident().to_string(), "MyStructFtl");

    // keyyed_idents are <StructName><Key>Ftl
    let mut key_names: Vec<String> = opts
        .keyyed_idents()
        .into_iter()
        .map(|k| k.to_string())
        .collect();
    key_names.sort();
    assert_eq!(
        key_names,
        vec![
            "MyStructErrorFtl".to_string(),
            "MyStructNoticeFtl".to_string()
        ]
    );

    // fields() filters out skipped fields
    let fields = opts.fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].ident().as_ref().unwrap().to_string(), "a");

    // 'this' flag was set
    assert!(opts.attr_args().is_this());
}

#[test]
#[should_panic(expected = "Unexpected mode")]
fn enum_display_strategy_invalid_value_panics() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(display = "invalid")]
        enum MyEnum {
            Unit,
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    // This call should panic due to invalid display value
    let _ = DisplayStrategy::from(&opts);
}

#[test]
#[should_panic(expected = "Unexpected mode")]
fn struct_display_strategy_invalid_value_panics() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(display = "bad")]
        struct MyStruct {
            a: i32,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    // This call should panic due to invalid display value
    let _ = DisplayStrategy::from(&opts);
}

#[test]
fn struct_fluent_parsing() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(this)]
        struct MyStruct {
            a: i32,
            #[fluent(skip)]
            b: String,
            #[fluent(choice)]
            c: bool,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    insta::assert_debug_snapshot!(&opts);
}

#[test]
fn enum_choice_parsing() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentChoice)]
        #[fluent_choice(serialize_all = "snake_case")]
        enum MyEnum {
            A,
            B,
        }
    };

    let opts = EnumChoiceOpts::from_derive_input(&input).expect("EnumChoiceOpts should parse");
    insta::assert_debug_snapshot!(&opts);
}
