use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::r#struct::StructKvOpts;
use es_fluent_core::validation::validate_struct_kv;
use syn::{DeriveInput, parse_quote};

#[test]
fn struct_kv_analysis_no_keys_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub struct User {
            name: String,
            age: u32,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "struct_kv_analysis_no_keys_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_kv_analysis_with_keys_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label", "description"])]
        pub struct User {
            name: String,
            age: u32,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "struct_kv_analysis_with_keys_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_kv_analysis_with_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub struct User {
            name: String,
            age: u32,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "struct_kv_analysis_with_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_kv_analysis_with_keys_and_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label"])]
        pub struct User {
            name: String,
            age: u32,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "struct_kv_analysis_with_keys_and_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_kv_analysis_with_keys_this_generates_expected_ftl_type_info() {
    // keys_this generates this_ftl on the generated KV enums (UserLabelKvFtl)
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label"])]
        pub struct User {
            name: String,
            age: u32,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "struct_kv_analysis_with_keys_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_kv_analysis_with_this_and_keys_this_generates_expected_ftl_type_info() {
    // this generates this_ftl on the original type (User)
    // keys_this generates this_ftl on the generated KV enums (UserLabelKvFtl)
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["label"])]
        pub struct User {
            name: String,
            age: u32,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    insta::assert_ron_snapshot!(
        "struct_kv_analysis_with_this_and_keys_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn validate_empty_struct_kv_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub struct EmptyStructKv {}
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    validate_struct_kv(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_empty_struct_kv_with_this_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub struct EmptyStructKv {}
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    validate_struct_kv(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_empty_struct_kv_with_keys_this_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub struct EmptyStructKv {}
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    validate_struct_kv(&opts, data).expect("Validation should succeed");
}
