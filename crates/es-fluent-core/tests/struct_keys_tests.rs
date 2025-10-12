use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::r#struct::StructKvOpts;
use syn::{DeriveInput, parse_quote};

#[test]
fn struct_analysis_with_keys_no_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["Error", "Notice"])]
        struct MyStruct {
            a: i32,
            #[fluent_kv(skip)]
            b: String,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos);

    insta::assert_ron_snapshot!(
        "struct_analysis_with_keys_no_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_analysis_with_multiple_keys_no_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["Error", "Notice", "Warning"])]
        struct MyStruct {
            first: String,
            second: bool,
            third: u8,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos);

    insta::assert_ron_snapshot!(
        "struct_analysis_with_multiple_keys_no_this_generates_expected_ftl_type_info",
        &infos
    );
}
