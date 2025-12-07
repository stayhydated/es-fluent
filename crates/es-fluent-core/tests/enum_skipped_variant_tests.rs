use darling::FromDeriveInput;
use es_fluent_core::analysis;
use es_fluent_core::options::r#enum::EnumOpts;
use syn::{DeriveInput, parse_quote};

#[test]
fn enum_analysis_with_skipped_variants_without_this() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        enum Mixed {
            // Unit variants
            #[fluent(skip)]
            UnitSkipped,
            UnitKept,

            // Struct variants
            #[fluent(skip)]
            DataSkipped { x: i32, y: i32 },
            DataKept {
                x: i32,
                #[fluent(skip)]
                secret: String,
                z: bool,
            },

            // Tuple variants
            #[fluent(skip)]
            TupSkipped(#[fluent(skip)] i64, String),
            TupKept(#[fluent(skip)] i64, String, u8),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    // Skipped variants are filtered during analysis.
    // This snapshot documents that only non-skipped variants are present.
    insta::assert_ron_snapshot!("enum_analysis_with_skipped_variants_without_this", &infos);
}

#[test]
fn enum_analysis_with_skipped_variants_with_this() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        #[fluent(this)]
        enum Mixed {
            // Unit variants
            #[fluent(skip)]
            UnitSkipped,
            UnitKept,

            // Struct variants
            #[fluent(skip)]
            DataSkipped { x: i32, y: i32 },
            DataKept {
                x: i32,
                #[fluent(skip)]
                secret: String,
                z: bool,
            },

            // Tuple variants
            #[fluent(skip)]
            TupSkipped(#[fluent(skip)] i64, String),
            TupKept(#[fluent(skip)] i64, String, u8),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    // Skipped variants are filtered during analysis.
    // With #[fluent(this)], the "this" variant appears first in both unit and struct/tuple groups, but only non-skipped variants are included.
    insta::assert_ron_snapshot!("enum_analysis_with_skipped_variants_with_this", &infos);
}

#[test]
fn enum_analysis_all_variants_skipped_yields_empty() {
    let input: DeriveInput = parse_quote! {
        #[derive(EsFluent)]
        enum AllSkipped {
            #[fluent(skip)]
            UnitA,
            #[fluent(skip)]
            Data { x: i32 },
            #[fluent(skip)]
            Tup(i32),
        }
    };

    let opts = EnumOpts::from_derive_input(&input).expect("EnumOpts should parse");
    let infos = analysis::analyze_enum(&opts);

    // When all variants are marked #[fluent(skip)], no FtlTypeInfo is produced.
    // This snapshot should be an empty list.
    insta::assert_ron_snapshot!("enum_analysis_all_variants_skipped_yields_empty", &infos);
}
