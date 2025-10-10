use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::{r#enum::EnumOpts, r#struct::StructOpts};
use syn::{DeriveInput, parse_quote};

#[test]
fn enum_analysis_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(this)]
        enum MyError {
            Unit,
            Data {
                a: i32,
                #[fluent(skip)]
                b: String,
            },
            Tuple(#[fluent(skip)] i32, String),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    insta::assert_ron_snapshot!("enum_analysis_generates_expected_ftl_type_info", &infos);
}

#[test]
fn struct_analysis_no_keys_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent]
        struct MyStruct {
            a: i32,
            #[fluent(skip)]
            b: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let infos = analysis::analyze_struct(&opts);

    insta::assert_ron_snapshot!(
        "struct_analysis_no_keys_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_analysis_with_keys_and_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(keys = ["Error", "Notice"], this)]
        struct MyStruct {
            a: i32,
            #[fluent(skip)]
            b: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let infos = analysis::analyze_struct(&opts);

    insta::assert_ron_snapshot!(
        "struct_analysis_with_keys_and_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn enum_analysis_only_unit_variants_no_this() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent]
        enum E {
            A,
            B,
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    insta::assert_ron_snapshot!("enum_analysis_only_unit_variants_no_this", &infos);
}

#[test]
fn enum_analysis_only_struct_and_tuple_variants_no_this() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent]
        enum E {
            Data { x: i32, y: i32 },
            Tup(String, #[fluent(skip)] bool, u8),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    insta::assert_ron_snapshot!(
        "enum_analysis_only_struct_and_tuple_variants_no_this",
        &infos
    );
}

#[test]
fn struct_analysis_all_fields_skipped_returns_empty() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(this, keys = ["A"])]
        struct S {
            #[fluent(skip)]
            a: i32,
            #[fluent(skip)]
            b: String,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let infos = analysis::analyze_struct(&opts);

    assert_eq!(&infos, &[]);
}

#[test]
fn struct_analysis_no_keys_with_this() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(this)]
        struct MyStruct {
            a: i32,
        }
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let infos = analysis::analyze_struct(&opts);

    insta::assert_ron_snapshot!("struct_analysis_no_keys_with_this", &infos);
}
