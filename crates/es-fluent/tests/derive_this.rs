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
struct TestKvStruct {
    field: String,
}

#[derive(EsFluentThis, EsFluentVariants)]
#[fluent_variants(keys = ["description"])]
#[fluent_this(members)]
#[allow(dead_code)]
enum TestKvEnum {
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
    // Generated name: TestKvStruct + Label + Variants = TestKvStructLabelVariants
    assert_eq!(
        TestKvStructLabelVariants::this_ftl(),
        "test_kv_struct_label_variants_this"
    );
}

#[test]
fn test_derive_this_variants() {
    // ThisFtl on generated variants enum for enum variants
    // Generated name: TestKvEnum + Description + Variants = TestKvEnumDescriptionVariants
    assert_eq!(
        TestKvEnumDescriptionVariants::this_ftl(),
        "test_kv_enum_description_variants_this"
    );
}
