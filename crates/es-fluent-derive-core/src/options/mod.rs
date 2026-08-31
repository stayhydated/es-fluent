//! This module provides types for parsing `es-fluent` attributes.

mod collection;
mod field;
mod meta;
mod scope;
mod variant;

pub mod choice;
pub mod r#enum;
pub mod label;
pub mod r#struct;

pub use collection::{
    EnumDataOptions, FilteredEnumDataOptions, GeneratedKeyList, GeneratedVariantsOptions,
    KeyedVariant, SkipDirective, Skippable, StructDataOptions, VariantFields,
    all_indexed_struct_items, all_variant_fields, collect_items, enum_items, filter_unskipped,
    filtered_enum_items, filtered_variant_fields, ftl_variants_ident, indexed_items,
    indexed_struct_items, indexed_unskipped, is_single_tuple_variant, keyed_base_idents,
    keyed_variant_idents, keyed_variants_base_idents, keyed_variants_idents, struct_items,
    variant_style, variants_enum_ident,
};
pub use field::{
    FieldArgumentDirective, FieldDirective, FieldValueDirective, FluentField, FluentFieldOpts,
    SkippableFieldOpts,
};
pub use meta::ValueAttr;
pub use scope::{
    DerivedNamespacedAttributeArgs, NamespacedAttributeArgs, ScopedAttributeArgs,
    VariantsFluentAttributeArgs,
};
pub use variant::{GeneratedVariantDirective, MessageVariantDirective};

pub(super) use field::FluentFieldAttributeArgs;
pub(super) use meta::{PresentFlag, string_literal_value};
pub(super) use variant::{KeyedVariantAttributeArgs, SkippedVariantAttributeArgs};

#[cfg(test)]
mod tests;
