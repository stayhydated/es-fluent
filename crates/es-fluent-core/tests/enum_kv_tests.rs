use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::r#enum::EnumKvOpts;
use es_fluent_core::validation::validate_enum_kv;
use syn::{DeriveInput, parse_quote};

#[test]
fn enum_kv_analysis_no_keys_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub enum Country {
            USA(USAState),
            Canada(CanadaProvince),
        }
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let mut infos = Vec::new();
    analysis::enum_kv::analyze_enum_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "enum_kv_analysis_no_keys_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn enum_kv_analysis_with_keys_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label", "description"])]
        pub enum Country {
            USA(USAState),
            Canada(CanadaProvince),
            Mexico(MexicoState),
        }
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let mut infos = Vec::new();
    analysis::enum_kv::analyze_enum_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "enum_kv_analysis_with_keys_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn enum_kv_analysis_with_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(this)]
        pub enum Country {
            USA(USAState),
            Canada(CanadaProvince),
        }
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let mut infos = Vec::new();
    analysis::enum_kv::analyze_enum_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "enum_kv_analysis_with_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn enum_kv_analysis_with_keys_and_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label"], this)]
        pub enum Country {
            USA(USAState),
            Canada(CanadaProvince),
        }
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let mut infos = Vec::new();
    analysis::enum_kv::analyze_enum_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "enum_kv_analysis_with_keys_and_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn enum_kv_analysis_with_skipped_variant_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub enum Country {
            USA(USAState),
            #[fluent_kv(skip)]
            Canada(CanadaProvince),
            Mexico(MexicoState),
        }
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let mut infos = Vec::new();
    analysis::enum_kv::analyze_enum_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "enum_kv_analysis_with_skipped_variant_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn validate_empty_enum_kv_without_this_produces_expected_error_message() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub enum EmptyEnumKv {}
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let data = match &input.data {
        syn::Data::Enum(data) => data,
        _ => unreachable!("input is an enum"),
    };

    let err = validate_enum_kv(&opts, data).expect_err("Expected validation error");

    insta::assert_ron_snapshot!(
        "validate_empty_enum_kv_without_this_produces_expected_error_message",
        err.to_string()
    );
}

#[test]
fn validate_empty_enum_kv_with_this_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(this)]
        pub enum EmptyEnumKv {}
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let data = match &input.data {
        syn::Data::Enum(data) => data,
        _ => unreachable!("input is an enum"),
    };

    validate_enum_kv(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_enum_kv_non_tuple_variant_produces_expected_error_message() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub enum InvalidEnum {
            USA(USAState),
            Canada { province: CanadaProvince },
        }
    };

    // This should fail at the darling parsing level because we only support enum_tuple
    let result = EnumKvOpts::from_derive_input(&input);
    assert!(result.is_err(), "Should fail to parse non-tuple enum");
}

#[test]
fn validate_enum_kv_unit_variant_produces_expected_error_message() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub enum InvalidEnum {
            USA(USAState),
            Unknown,
        }
    };

    // This should fail at the darling parsing level because we only support enum_tuple
    let result = EnumKvOpts::from_derive_input(&input);
    assert!(
        result.is_err(),
        "Should fail to parse enum with unit variant"
    );
}
