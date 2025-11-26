use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::r#struct::StructKvOpts;
use syn::{DeriveInput, parse_quote};

#[test]
fn struct_analysis_with_multiple_keys_and_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(this, keys = ["error", "notice", "warning"])]
        struct MyStruct {
            first: String,
            #[fluent_kv(skip)]
            secret: bool,
            second: i32,
            third: u8,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    // Expect:
    // - One FtlTypeInfo per key: MyStructErrorFtl, MyStructNoticeFtl, MyStructWarningFtl
    //   - Each contains a "this" variant (name = <KeyedEnumIdent>) plus variants for fields
    //     [first, second, third] (secret is skipped)
    // - One extra "main struct" FtlTypeInfo for MyStruct with a single "this" variant
    insta::assert_ron_snapshot!(
        "struct_analysis_with_multiple_keys_and_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_analysis_with_multiple_keys_and_this_more_fields_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(this, keys = ["primary", "secondary"])]
        struct Profile {
            id: u64,
            username: String,
            #[fluent_kv(skip)]
            password_hash: String,
            email: String,
            active: bool,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    // Expect:
    // - Keys produce: ProfilePrimaryFtl and ProfileSecondaryFtl
    //   - Each has "this" as first variant, then [id, username, email, active]
    // - Main "Profile" enum with only the "this" variant
    insta::assert_ron_snapshot!(
        "struct_analysis_with_multiple_keys_and_this_more_fields_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn struct_analysis_with_two_keys_out_of_order_and_this_generates_expected_ftl_type_info() {
    // Intentionally out-of-order keys to ensure iteration preserves user order
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluentKv)]
        #[fluent_kv(this, keys = ["zed", "alpha"])]
        struct SystemConfig {
            host: String,
            port: u16,
            #[fluent_kv(skip)]
            deprecated_field: i32,
            use_tls: bool,
        }
    };

    let opts = StructKvOpts::from_derive_input(&input).expect("StructKvOpts should parse");
    let mut infos = Vec::new();
    analysis::struct_kv::analyze_struct_kv(&opts, &mut infos).unwrap();

    // Expect:
    // - Order of FtlTypeInfo matches keys declaration: SystemConfigZedFtl then SystemConfigAlphaFtl
    // - Each keyed Ftl contains "this" then [host, port, use_tls]
    // - Final main FtlTypeInfo for SystemConfig with a single "this" variant
    insta::assert_ron_snapshot!(
        "struct_analysis_with_two_keys_out_of_order_and_this_generates_expected_ftl_type_info",
        &infos
    );
}
