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
        #[fluent_kv(keys = ["label"])]
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
fn enum_kv_analysis_with__generates_expected_ftl_type_info() {
    //  generates this_ftl on the original type (Country)
    // Use this when the original type does NOT have EsFluent with this
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label"], )]
        pub enum Country {
            USA(USAState),
            Canada(CanadaProvince),
        }
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let mut infos = Vec::new();
    analysis::enum_kv::analyze_enum_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "enum_kv_analysis_with__generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn enum_kv_analysis_with_this_and__generates_expected_ftl_type_info() {
    // this generates this_ftl on the generated KV enums (CountryLabelKvFtl)
    //  generates this_ftl on the original type (Country)
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label"], )]
        pub enum Country {
            USA(USAState),
            Canada(CanadaProvince),
        }
    };

    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    let mut infos = Vec::new();
    analysis::enum_kv::analyze_enum_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "enum_kv_analysis_with_this_and__generates_expected_ftl_type_info",
        &infos
    );
}


#[test]
fn validate_empty_enum_kv_with_this_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
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
fn validate_enum_kv_non_tuple_variant_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub enum InvalidEnum {
            USA(USAState),
            Canada { province: CanadaProvince },
        }
    };

    // This should now succeed because we relaxed validation to support all variants
    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    validate_enum_kv(&opts, match &input.data {
        syn::Data::Enum(data) => data,
        _ => unreachable!(),
    }).expect("Validation should succeed");
}

#[test]
fn validate_enum_kv_unit_variant_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub enum InvalidEnum {
            USA(USAState),
            Unknown,
        }
    };

    // This should now succeed because we relaxed validation to support all variants
    let opts = EnumKvOpts::from_derive_input(&input).expect("EnumKvOpts should parse");
    validate_enum_kv(&opts, match &input.data {
        syn::Data::Enum(data) => data,
        _ => unreachable!(),
    }).expect("Validation should succeed");
}
