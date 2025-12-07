use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::r#enum::EnumOpts;
use syn::{DeriveInput, parse_quote};

#[test]
fn enum_mixed_variants_analysis_without_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        enum Mixed {
            // unit variants
            UnitA,
            UnitB,
            // struct variant with one skipped field
            Data {
                x: i32,
                #[fluent(skip)]
                secret: String,
                y: bool,
            },
            // tuple variant with first field skipped
            Other(#[fluent(skip)] i64, String, u8),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    insta::assert_ron_snapshot!(
        "enum_mixed_variants_analysis_without_this_generates_expected_ftl_type_info",
        &infos
    );
}

#[test]
fn enum_mixed_variants_analysis_with_this_generates_expected_ftl_type_info() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(this)]
        enum Mixed {
            // unit variants
            UnitA,
            UnitB,
            // struct variant with one skipped field
            Data {
                x: i32,
                #[fluent(skip)]
                secret: String,
                y: bool,
            },
            // tuple variant with first field skipped
            Other(#[fluent(skip)] i64, String, u8),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    insta::assert_ron_snapshot!(
        "enum_mixed_variants_analysis_with_this_generates_expected_ftl_type_info",
        &infos
    );
}
