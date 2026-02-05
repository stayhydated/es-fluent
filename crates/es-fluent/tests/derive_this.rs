use es_fluent::{EsFluent, EsFluentThis, EsFluentVariants, ThisFtl};

#[derive(EsFluent, EsFluentThis)]
#[fluent_this(origin)]
struct TestStruct {
    field: String,
}

#[derive(EsFluentThis, EsFluentVariants)]
#[fluent_variants(keys = ["label"])]
#[fluent_this(members)]
#[allow(dead_code)]
struct TestVariantsStruct {
    field: String,
}

#[derive(EsFluentThis, EsFluentVariants)]
#[fluent_variants(keys = ["description"])]
#[fluent_this(members)]
#[allow(dead_code)]
enum TestVariantsEnum {
    VariantA,
}

#[test]
fn test_derive_this_struct() {
    // Basic ThisFtl on struct
    assert_eq!(TestStruct::this_ftl(), "test_struct_this");
}

#[test]
fn test_derive_this_fields() {
    // ThisFtl on generated variants enum for struct fields
    // Generated name: TestVariantsStruct + Label + Variants = TestVariantsStructLabelVariants
    assert_eq!(
        TestVariantsStructLabelVariants::this_ftl(),
        "test_variants_struct_label_variants_this"
    );
}

#[test]
fn test_derive_this_variants() {
    // ThisFtl on generated variants enum for enum variants
    // Generated name: TestVariantsEnum + Description + Variants = TestVariantsEnumDescriptionVariants
    assert_eq!(
        TestVariantsEnumDescriptionVariants::this_ftl(),
        "test_variants_enum_description_variants_this"
    );
}
