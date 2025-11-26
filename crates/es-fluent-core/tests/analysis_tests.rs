use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::{
    r#enum::EnumOpts,
    r#struct::{StructKvOpts, StructOpts},
};
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
fn struct_analysis_tuple_struct_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent]
        struct TupleStruct(String, #[fluent(skip)] i32, bool);
    };

    let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
    let infos = analysis::analyze_struct(&opts);

    insta::assert_ron_snapshot!(
        "struct_analysis_tuple_struct_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_analysis_with_keys_and_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(keys = ["error", "notice"], this)]
        struct MyStruct {
            a: i32,
            #[fluent_kv(skip)]
            b: String,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

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
        #[derive(EsFluentKv)]
        #[fluent_kv(this, keys = ["a"])]
        struct S {
            #[fluent_kv(skip)]
            a: i32,
            #[fluent_kv(skip)]
            b: String,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

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
