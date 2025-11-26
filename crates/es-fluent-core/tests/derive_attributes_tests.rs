use darling::FromDeriveInput;
use es_fluent_core::options::{
    r#enum::{EnumChoiceOpts, EnumOpts},
    r#struct::{StructKvOpts, StructOpts},
};
use syn::{DeriveInput, parse_quote};

/// EsFluent on enums: default display (fluent), no `this`, no enum-level `choice`
#[test]
fn es_fluent_enum_attributes_default_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent] // default: display = "fluent", no this, no choice
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
    insta::assert_debug_snapshot!(
        "es_fluent_enum_attributes_default_snapshot__analysis",
        &opts
    );
}

/// EsFluent on enums: override display to std, enable `this`, and enum-level `choice` flag
#[test]
fn es_fluent_enum_attributes_std_this_choice_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(display = "std", this, choice)]
        enum Status {
            // unit
            Ok,
            // tuple with a choice field and a skipped field
            Mixed(#[fluent(choice)] Severity, #[fluent(skip)] i32),
            // struct with one choice field
            Info {
                #[fluent(choice)]
                level: Severity,
                message: String,
            }
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    insta::assert_debug_snapshot!(
        "es_fluent_enum_attributes_std_this_choice_snapshot__analysis",
        &opts
    );
}

/// EsFluent on structs: default display (fluent), with `this`, and derive list present
#[test]
fn es_fluent_struct_attributes_this_with_derive_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(this, derive(Debug, Clone))]
        struct Person {
            name: String,
            #[fluent(skip)]
            password_hash: String,
            #[fluent(choice)]
            gender: Gender,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    insta::assert_debug_snapshot!(
        "es_fluent_struct_attributes_this_with_derive_snapshot__opts",
        &opts
    );
}

/// EsFluent on structs: override display to std, no `this`, exercise default and choice fields
#[test]
fn es_fluent_struct_attributes_std_with_default_and_choice_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(display = "std")]
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
    insta::assert_debug_snapshot!(
        "es_fluent_struct_attributes_std_with_default_and_choice_snapshot__opts",
        &opts
    );
}

/// EsFluentKv on structs: no keys provided (single FTL enum), default display is std
#[test]
fn es_fluent_kv_attributes_no_keys_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv] // default: display = "std", no keys, no this
        struct Config {
            host: String,
            port: u16,
            #[fluent_kv(skip)]
            deprecated: bool,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    insta::assert_debug_snapshot!("es_fluent_kv_attributes_no_keys_snapshot__analysis", &opts);
}

/// EsFluentKv on structs: with keys, `this`, `derive` list, and a default field
#[test]
fn es_fluent_kv_attributes_keys_this_derive_default_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(this, keys = ["primary", "secondary"], derive(Debug, PartialEq))]
        struct Profile {
            #[fluent_kv(default)]
            id: u64,
            username: String,
            #[fluent_kv(skip)]
            secret_token: String,
            active: bool,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    insta::assert_debug_snapshot!(
        "es_fluent_kv_attributes_keys_this_derive_default_snapshot__analysis",
        &opts
    );
}

/// EsFluentChoice on enums: no serialize_all provided (identity)
#[test]
fn es_fluent_choice_attributes_none_snapshot() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentChoice)]
        #[fluent_choice] // serialize_all not set
        enum Gender {
            Male,
            Female,
            Other,
        }
    };

    let opts = EnumChoiceOpts::from_derive_input(&input).expect("EnumChoiceOpts should parse");
    insta::assert_debug_snapshot!("es_fluent_choice_attributes_none_snapshot__opts", &opts);
}

/// EsFluentChoice on enums: serialize_all = "snake_case"
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
    insta::assert_debug_snapshot!(
        "es_fluent_choice_attributes_snake_case_snapshot__opts",
        &opts
    );
}

/// EsFluentChoice on enums: serialize_all = "SCREAMING_SNAKE_CASE"
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
    insta::assert_debug_snapshot!(
        "es_fluent_choice_attributes_screaming_snake_snapshot__opts",
        &opts
    );
}
