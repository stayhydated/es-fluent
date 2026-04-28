use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::{
    EnumDataOptions as _, FluentField as _, GeneratedVariantsOptions as _, StructDataOptions as _,
    VariantFields as _,
    r#enum::{EnumChoiceOpts, EnumOpts},
    r#struct::{StructOpts, StructVariantsOpts},
};
use syn::{DeriveInput, parse_quote};

fn assert_no_generics(generics: &syn::Generics) {
    assert!(generics.params.is_empty());
    assert!(generics.where_clause.is_none());
}

fn derive_names(paths: &darling::util::PathList) -> Vec<String> {
    paths
        .iter()
        .map(|path| {
            path.segments
                .last()
                .expect("derive path should have a segment")
                .ident
                .to_string()
        })
        .collect()
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
fn es_fluent_enum_attributes_default_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]

        enum ApiError {
            NotFound,
            PermissionDenied,
            Data {
                id: u64,
                #[fluent(skip)]
                internal: bool,
            },
            TupleVariant(String, #[fluent(skip)] i32),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    assert_eq!(opts.ident().to_string(), "ApiError");
    assert_no_generics(opts.generics());
    assert_eq!(opts.variants().len(), 4);
    assert!(opts.attr_args().resource().is_none());
    assert!(opts.attr_args().domain().is_none());
    assert!(!opts.attr_args().skip_inventory());
    assert!(opts.attr_args().namespace().is_none());

    let data = opts
        .variants()
        .into_iter()
        .find(|variant| *variant.ident() == "Data")
        .expect("Data variant should exist");
    assert!(matches!(data.style(), darling::ast::Style::Struct));
    assert_eq!(data.fields().len(), 1);
    assert_eq!(data.all_fields().len(), 2);
    assert_eq!(
        data.fields()[0].ident().expect("named field").to_string(),
        "id"
    );
    assert!(data.all_fields()[1].is_skipped());

    let tuple = opts
        .variants()
        .into_iter()
        .find(|variant| *variant.ident() == "TupleVariant")
        .expect("TupleVariant should exist");
    assert!(matches!(tuple.style(), darling::ast::Style::Tuple));
    assert_eq!(tuple.fields().len(), 1);
    assert_eq!(tuple.all_fields().len(), 2);
    assert!(tuple.fields()[0].ident().is_none());
    assert!(tuple.all_fields()[1].is_skipped());
}

#[test]
fn es_fluent_enum_attributes_this_choice_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        enum Status {
            Ok,
            Mixed(#[fluent(choice)] Severity, #[fluent(skip)] i32),
            Info {
                #[fluent(choice)]
                level: Severity,
                message: String,
            }
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    assert_eq!(opts.ident().to_string(), "Status");
    assert_no_generics(opts.generics());
    assert_eq!(opts.variants().len(), 3);

    let mixed = opts
        .variants()
        .into_iter()
        .find(|variant| *variant.ident() == "Mixed")
        .expect("Mixed variant should exist");
    assert!(matches!(mixed.style(), darling::ast::Style::Tuple));
    assert_eq!(mixed.fields().len(), 1);
    assert_eq!(mixed.all_fields().len(), 2);
    assert!(mixed.fields()[0].is_choice());
    assert!(mixed.all_fields()[1].is_skipped());

    let info = opts
        .variants()
        .into_iter()
        .find(|variant| *variant.ident() == "Info")
        .expect("Info variant should exist");
    assert!(matches!(info.style(), darling::ast::Style::Struct));
    assert_eq!(info.fields().len(), 2);
    assert_eq!(
        info.fields()[0].ident().expect("named field").to_string(),
        "level"
    );
    assert!(info.fields()[0].is_choice());
    assert_eq!(
        info.fields()[1].ident().expect("named field").to_string(),
        "message"
    );
    assert!(!info.fields()[1].is_choice());
}

#[test]
fn es_fluent_struct_attributes_this_with_derive_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(derive(Debug, Clone))]
        struct Person {
            name: String,
            #[fluent(skip)]
            password_hash: String,
            #[fluent(choice)]
            gender: Gender,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(opts.ident().to_string(), "Person");
    assert_no_generics(opts.generics());
    assert_eq!(
        derive_names(opts.attr_args().derive()),
        vec!["Debug", "Clone"]
    );
    assert!(opts.attr_args().namespace().is_none());

    let fields = opts.fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].ident().expect("named field").to_string(), "name");
    assert!(!fields[0].is_choice());
    assert_eq!(
        fields[1].ident().expect("named field").to_string(),
        "gender"
    );
    assert!(fields[1].is_choice());

    let all_fields = opts.all_indexed_fields();
    assert_eq!(all_fields.len(), 3);
    assert_eq!(
        all_fields[1].1.ident().expect("named field").to_string(),
        "password_hash"
    );
    assert!(all_fields[1].1.is_skipped());
}

#[test]
fn es_fluent_struct_attributes_default_and_choice_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]

        struct Label {
            #[fluent(default)]
            text: String,
            #[fluent(choice)]
            style: Emphasis,
            #[fluent(skip)]
            developer_only_flag: bool,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    assert_eq!(opts.ident().to_string(), "Label");
    assert_no_generics(opts.generics());
    assert!(derive_names(opts.attr_args().derive()).is_empty());
    assert!(opts.attr_args().namespace().is_none());

    let fields = opts.fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].ident().expect("named field").to_string(), "text");
    assert!(fields[0].is_default());
    assert!(!fields[0].is_choice());
    assert_eq!(fields[1].ident().expect("named field").to_string(), "style");
    assert!(!fields[1].is_default());
    assert!(fields[1].is_choice());

    let all_fields = opts.all_indexed_fields();
    assert_eq!(all_fields.len(), 3);
    assert_eq!(
        all_fields[2].1.ident().expect("named field").to_string(),
        "developer_only_flag"
    );
    assert!(all_fields[2].1.is_skipped());
}

#[test]
fn es_fluent_variants_attributes_no_keys_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentVariants)]

        struct Config {
            host: String,
            port: u16,
            #[fluent_variants(skip)]
            deprecated: bool,
        }
    };

    let opts =
        StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts should parse");
    assert_eq!(opts.ident().to_string(), "Config");
    assert_no_generics(opts.generics());
    assert_eq!(opts.ftl_enum_ident().to_string(), "ConfigVariants");
    assert!(
        opts.keyed_idents()
            .expect("keyed idents should parse")
            .is_empty()
    );
    assert!(derive_names(opts.attr_args().derive()).is_empty());
    assert!(opts.attr_args().key_strings().is_none());
    assert!(opts.attr_args().namespace().is_none());

    let fields = opts.fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].ident().expect("named field").to_string(), "host");
    assert_eq!(fields[1].ident().expect("named field").to_string(), "port");

    let all_fields = opts.all_indexed_fields();
    assert_eq!(all_fields.len(), 3);
    assert_eq!(
        all_fields[2].1.ident().expect("named field").to_string(),
        "deprecated"
    );
    assert!(all_fields[2].1.is_skipped());
}

#[test]
fn es_fluent_variants_attributes_keys_this_derive_default_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentVariants)]
        #[fluent_variants(keys = ["primary", "secondary"], derive(Debug, PartialEq))]
        struct Profile {
            id: u64,
            username: String,
            #[fluent_variants(skip)]
            secret_token: String,
            active: bool,
        }
    };

    let opts =
        StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts should parse");
    assert_eq!(opts.ident().to_string(), "Profile");
    assert_no_generics(opts.generics());
    assert_eq!(opts.ftl_enum_ident().to_string(), "ProfileVariants");
    assert_eq!(
        opts.attr_args().key_strings(),
        Some(vec!["primary".to_string(), "secondary".to_string()])
    );
    assert_eq!(
        derive_names(opts.attr_args().derive()),
        vec!["Debug", "PartialEq"]
    );

    let keyed_idents: Vec<String> = opts
        .keyed_idents()
        .expect("keyed idents should parse")
        .into_iter()
        .map(|ident| ident.to_string())
        .collect();
    assert_eq!(
        keyed_idents,
        vec!["ProfilePrimaryVariants", "ProfileSecondaryVariants"]
    );

    let fields = opts.fields();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].ident().expect("named field").to_string(), "id");
    assert_eq!(
        fields[1].ident().expect("named field").to_string(),
        "username"
    );
    assert_eq!(
        fields[2].ident().expect("named field").to_string(),
        "active"
    );
}

#[test]
fn es_fluent_choice_attributes_none_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentChoice)]
        #[fluent_choice]
        enum Gender {
            Male,
            Female,
            Other,
        }
    };

    let opts = EnumChoiceOpts::from_derive_input(&input).expect("EnumChoiceOpts should parse");
    assert_eq!(opts.ident().to_string(), "Gender");
    assert_no_generics(opts.generics());
    assert_eq!(ignored_enum_variant_count(opts.data()), 3);
    assert_eq!(opts.attr_args().serialize_all().as_deref(), None);
}

#[test]
fn es_fluent_choice_attributes_snake_case_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentChoice)]
        #[fluent_choice(serialize_all = "snake_case")]
        enum Severity {
            VeryLow,
            Low,
            Medium,
            High,
            VeryHigh,
        }
    };

    let opts = EnumChoiceOpts::from_derive_input(&input).expect("EnumChoiceOpts should parse");
    assert_eq!(opts.ident().to_string(), "Severity");
    assert_no_generics(opts.generics());
    assert_eq!(ignored_enum_variant_count(opts.data()), 5);
    assert_eq!(
        opts.attr_args().serialize_all().as_deref(),
        Some("snake_case")
    );
}

#[test]
fn es_fluent_choice_attributes_screaming_snake_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentChoice)]
        #[fluent_choice(serialize_all = "SCREAMING_SNAKE_CASE")]
        enum Emphasis {
            None,
            Light,
            Medium,
            Strong,
        }
    };

    let opts = EnumChoiceOpts::from_derive_input(&input).expect("EnumChoiceOpts should parse");
    assert_eq!(opts.ident().to_string(), "Emphasis");
    assert_no_generics(opts.generics());
    assert_eq!(ignored_enum_variant_count(opts.data()), 4);
    assert_eq!(
        opts.attr_args().serialize_all().as_deref(),
        Some("SCREAMING_SNAKE_CASE")
    );
}
