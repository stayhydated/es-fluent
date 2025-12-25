use darling::FromDeriveInput;
use es_fluent_core::options::r#enum::EnumOpts;
use es_fluent_core::options::r#struct::{StructKvOpts, StructOpts};
use es_fluent_core::validation::{validate_enum, validate_struct, validate_struct_kv};
use syn::{DataStruct, DeriveInput, parse_quote};

#[test]
fn validate_struct_multiple_defaults_produces_expected_error_message() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct TestStruct {
            #[fluent(default)]
            field1: String,
            #[fluent(default)]
            field2: i32,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let data = DataStruct {
        struct_token: Default::default(),
        fields: syn::Fields::Named(parse_quote! { { field1: String, field2: i32 } }),
        semi_token: None,
    };

    let err = validate_struct(&opts, &data).expect_err("Expected validation error");

    insta::assert_ron_snapshot!(
        "validate_struct_multiple_defaults_produces_expected_error_message",
        err.to_string()
    );
}

#[test]
fn validate_struct_multiple_defaults_tuple_struct_produces_expected_error_message() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct TestTupleStruct(
            #[fluent(default)]
            String,
            #[fluent(default)]
            i32,
        );
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    let err = validate_struct(&opts, data).expect_err("Expected validation error");

    insta::assert_ron_snapshot!(
        "validate_struct_multiple_defaults_tuple_struct_produces_expected_error_message",
        err.to_string()
    );
}

#[test]
fn validate_empty_enum_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub enum EmptyEnum {}
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let data = match &input.data {
        syn::Data::Enum(data) => data,
        _ => unreachable!("input is an enum"),
    };

    validate_enum(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_empty_enum_with_this_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub enum EmptyEnum {}
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let data = match &input.data {
        syn::Data::Enum(data) => data,
        _ => unreachable!("input is an enum"),
    };

    validate_enum(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_empty_struct_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct EmptyStruct;
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    validate_struct(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_empty_struct_with_this_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct EmptyStruct;
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    validate_struct(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_empty_named_struct_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        pub struct EmptyStruct {}
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    validate_struct(&opts, data).expect("Validation should succeed");
}

#[test]
fn validate_empty_struct_kv_succeeds() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        pub struct EmptyStructKv;
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
        pub struct EmptyStructKv;
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let data = match &input.data {
        syn::Data::Struct(data) => data,
        _ => unreachable!("input is a struct"),
    };

    validate_struct_kv(&opts, data).expect("Validation should succeed");
}
