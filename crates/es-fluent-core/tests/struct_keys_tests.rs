use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::r#struct::StructOpts;
use syn::{DeriveInput, parse_quote};

#[test]
fn struct_analysis_with_keys_no_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(keys = ["Error", "Notice"])]
        struct MyStruct {
            a: i32,
            #[fluent(skip)]
            b: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let infos = analysis::analyze_struct(&opts);

    insta::assert_ron_snapshot!(
        "struct_analysis_with_keys_no_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_analysis_with_multiple_keys_no_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(keys = ["Error", "Notice", "Warning"])]
        struct MyStruct {
            first: String,
            second: bool,
            third: u8,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let infos = analysis::analyze_struct(&opts);

    insta::assert_ron_snapshot!(
        "struct_analysis_with_multiple_keys_no_this_generates_expected_ftl_type_info",
        &infos
    );
}
