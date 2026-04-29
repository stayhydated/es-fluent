use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::{
    EnumDataOptions as _, FluentField as _, GeneratedVariantsOptions as _, StructDataOptions as _,
    VariantFields as _,
    r#enum::{EnumChoiceOpts, EnumOpts},
    r#struct::{StructOpts, StructVariantsOpts},
};
use es_fluent_shared::namespace::NamespaceRule;
use syn::{DeriveInput, parse_quote};

fn assert_no_generics(generics: &syn::Generics) {
    assert!(generics.params.is_empty());
    assert!(generics.where_clause.is_none());
}

fn ignored_enum_variant_count(
    data: &darling::ast::Data<darling::util::Ignored, darling::util::Ignored>,
) -> usize {
    match data {
        darling::ast::Data::Enum(variants) => variants.len(),
        darling::ast::Data::Struct(_) => panic!("expected enum data"),
    }
}

#[test]
fn enum_variants_and_fields_skipping_and_choice() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]

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

    let data = variants
        .into_iter()
        .find(|variant| *variant.ident() == "Data")
        .expect("Data variant present");
    assert!(matches!(data.style(), darling::ast::Style::Struct));
    let data_fields = data.fields();
    assert_eq!(data_fields.len(), 1, "Only non-skipped field remains");
    assert_eq!(
        data_fields[0].ident().expect("named field").to_string(),
        "a",
        "Expected remaining field to be 'a'"
    );
    assert!(
        data_fields[0].is_choice(),
        "Field 'a' should be marked as choice"
    );

    let tuple = opts
        .variants()
        .into_iter()
        .find(|variant| *variant.ident() == "Tuple")
        .expect("Tuple variant present");
    assert!(matches!(tuple.style(), darling::ast::Style::Tuple));
    assert_eq!(tuple.all_fields().len(), 2, "All tuple fields parsed");
    assert_eq!(tuple.fields().len(), 1, "One tuple field was skipped");
    assert!(tuple.fields()[0].ident().is_none());

    let unit = opts
        .variants()
        .into_iter()
        .find(|variant| *variant.ident() == "Unit")
        .expect("Unit variant present");
    assert!(matches!(unit.style(), darling::ast::Style::Unit));
    assert!(unit.fields().is_empty());
}

#[test]
fn enum_variant_level_arg_name_is_rejected() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        enum MyEnum {
            #[fluent(arg_name = "value")]
            Tuple(String),
        }
    };

    let err = EnumOpts::from_derive_input(&input).expect_err("Expected parse error");
    assert!(err.to_string().contains("arg_name"));
}

#[test]
fn enum_tuple_field_arg_name_parsing() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        enum MyEnum {
            Tuple(#[fluent(arg_name = "value")] String),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let variants = opts.variants();
    let tuple = variants
        .into_iter()
        .find(|variant| *variant.ident() == "Tuple")
        .expect("Tuple variant present");

    let fields = tuple.all_fields();
    let field_arg_name = fields[0].arg_name().expect("field arg_name should parse");
    assert_eq!(field_arg_name, "value".to_string());
}

#[test]
fn enum_named_field_arg_name_parsing() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        enum MyEnum {
            Named {
                #[fluent(arg_name = "display_value")]
                value: String,
            },
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let variants = opts.variants();
    let named = variants
        .into_iter()
        .find(|variant| *variant.ident() == "Named")
        .expect("Named variant present");

    let fields = named.all_fields();
    let field_arg_name = fields[0].arg_name().expect("field arg_name should parse");
    assert_eq!(field_arg_name, "display_value".to_string());
}

#[test]
fn struct_variants_keys_parsing_and_field_skipping() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentVariants)]
        #[fluent_variants(keys = ["error", "notice"])]
        struct MyStruct {
            a: i32,
            #[fluent_variants(skip)]
            b: String,
        }
    };

    let opts =
        StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts should parse");

    assert_eq!(opts.ftl_enum_ident().to_string(), "MyStructVariants");
    assert_eq!(
        opts.attr_args().key_strings(),
        Some(vec!["error".to_string(), "notice".to_string()])
    );

    let mut key_names: Vec<String> = opts
        .keyed_idents()
        .expect("keyed identifiers should parse")
        .into_iter()
        .map(|ident| ident.to_string())
        .collect();
    key_names.sort();
    assert_eq!(
        key_names,
        vec![
            "MyStructErrorVariants".to_string(),
            "MyStructNoticeVariants".to_string()
        ]
    );

    let fields = opts.fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].ident().expect("named field").to_string(), "a");
}

#[test]
fn struct_variants_keys_must_be_lowercase_snake_case() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentVariants)]
        #[fluent_variants(keys = ["NotSnake"])]
        struct MyStruct {
            field: i32,
        }
    };

    let opts =
        StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts should parse");
    let err = opts
        .keyed_idents()
        .expect_err("Non-snake_case keys should be rejected");

    let err_message = err.to_string();
    assert!(
        err_message.contains("lowercase snake_case"),
        "Unexpected error message: {err_message}"
    );
}

#[test]
fn struct_fluent_parsing() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        struct MyStruct {
            a: i32,
            #[fluent(skip)]
            b: String,
            #[fluent(choice)]
            c: bool,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(opts.ident().to_string(), "MyStruct");
    assert_no_generics(opts.generics());
    assert!(opts.attr_args().derive().is_empty());
    assert!(opts.attr_args().namespace().is_none());

    let fields = opts.fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].ident().expect("named field").to_string(), "a");
    assert!(!fields[0].is_choice());
    assert_eq!(fields[1].ident().expect("named field").to_string(), "c");
    assert!(fields[1].is_choice());

    let all_fields = opts.all_indexed_fields();
    assert_eq!(all_fields.len(), 3);
    assert_eq!(
        all_fields[1].1.ident().expect("named field").to_string(),
        "b"
    );
    assert!(all_fields[1].1.is_skipped());
}

#[test]
fn struct_tuple_fields_parsing() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]

        struct TupleStruct(#[fluent(skip)] i32, String, #[fluent(choice)] bool);
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let fields = opts.fields();
    assert_eq!(fields.len(), 2, "Only non-skipped fields remain");
    assert!(fields.iter().all(|field| field.ident().is_none()));

    let indexed_fields = opts.indexed_fields();
    assert_eq!(indexed_fields.len(), 2, "Two indexed fields remain");

    let (first_index, first_field) = &indexed_fields[0];
    assert_eq!(*first_index, 1);
    assert_eq!(first_field.fluent_arg_name(*first_index), "f1");
    assert!(!first_field.is_choice());

    let (second_index, second_field) = &indexed_fields[1];
    assert_eq!(*second_index, 2);
    assert_eq!(second_field.fluent_arg_name(*second_index), "f2");
    assert!(second_field.is_choice());
}

#[test]
fn struct_named_field_arg_name_parsing() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        struct MyStruct {
            #[fluent(arg_name = "display_name")]
            name: String,
            value: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let indexed_fields = opts.indexed_fields();
    assert_eq!(
        indexed_fields[0].1.fluent_arg_name(indexed_fields[0].0),
        "display_name"
    );
    assert_eq!(
        indexed_fields[1].1.fluent_arg_name(indexed_fields[1].0),
        "value"
    );
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
    assert_eq!(opts.ident().to_string(), "MyEnum");
    assert_no_generics(opts.generics());
    assert_eq!(ignored_enum_variant_count(opts.data()), 2);
    assert_eq!(
        opts.attr_args().serialize_all().as_deref(),
        Some("snake_case")
    );
}

#[test]
fn struct_fluent_with_namespace_literal() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(namespace = "ui")]
        struct Button {
            label: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(opts.ident().to_string(), "Button");
    assert_no_generics(opts.generics());
    assert_eq!(opts.fields().len(), 1);
    assert_eq!(
        opts.fields()[0].ident().expect("named field").to_string(),
        "label"
    );
    assert!(matches!(
        opts.attr_args().namespace(),
        Some(NamespaceRule::Literal(value)) if value == "ui"
    ));
}

#[test]
fn struct_fluent_with_namespace_file() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(namespace = file)]
        struct Dialog {
            title: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(opts.ident().to_string(), "Dialog");
    assert_no_generics(opts.generics());
    assert_eq!(opts.fields().len(), 1);
    assert_eq!(
        opts.fields()[0].ident().expect("named field").to_string(),
        "title"
    );
    assert!(matches!(
        opts.attr_args().namespace(),
        Some(NamespaceRule::File)
    ));
}

#[test]
fn struct_fluent_with_namespace_file_relative() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(namespace(file(relative)))]
        struct Modal {
            content: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(opts.ident().to_string(), "Modal");
    assert_no_generics(opts.generics());
    assert_eq!(opts.fields().len(), 1);
    assert_eq!(
        opts.fields()[0].ident().expect("named field").to_string(),
        "content"
    );
    assert!(matches!(
        opts.attr_args().namespace(),
        Some(NamespaceRule::FileRelative)
    ));
}

#[test]
fn struct_fluent_with_namespace_folder() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(namespace = folder)]
        struct FolderModal {
            content: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert!(matches!(
        opts.attr_args().namespace(),
        Some(NamespaceRule::Folder)
    ));
}

#[test]
fn struct_fluent_with_namespace_folder_relative() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(namespace(folder(relative)))]
        struct FolderRelativeModal {
            content: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert!(matches!(
        opts.attr_args().namespace(),
        Some(NamespaceRule::FolderRelative)
    ));
}

#[test]
fn enum_fluent_with_namespace_literal() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(namespace = "errors")]
        enum ApiError {
            NotFound,
            Unauthorized,
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    assert_eq!(opts.ident().to_string(), "ApiError");
    assert_no_generics(opts.generics());
    assert_eq!(opts.base_key(), "api_error");
    assert_eq!(opts.variants().len(), 2);
    assert!(matches!(
        opts.attr_args().namespace(),
        Some(NamespaceRule::Literal(value)) if value == "errors"
    ));
    assert!(opts.attr_args().resource().is_none());
    assert!(opts.attr_args().domain().is_none());
    assert!(!opts.attr_args().skip_inventory());
}
