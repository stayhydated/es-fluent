use darling::FromDeriveInput;
use es_fluent_core::options::r#struct::StructOpts;
use es_fluent_core::validation::validate_struct;
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
