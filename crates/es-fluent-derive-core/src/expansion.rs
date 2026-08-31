//! Validated expansion models consumed by proc-macro token emission.

mod choice;
mod input;
mod label;
mod message;
mod message_enum;
mod message_struct;
mod validation;
mod variants;

pub use choice::EsFluentChoiceExpansion;
pub use input::{DeriveFamily, ExpansionError, ExpansionResult, ValidatedDeriveInput};
pub use label::EsFluentLabelExpansion;
pub use message::EsFluentExpansion;
pub use message_enum::{
    EsFluentEnumExpansion, EsFluentEnumVariantShape, EsFluentLocalizedVariant,
    EsFluentMessageVariant, EsFluentNamedField, EsFluentSkippedVariant, EsFluentTupleField,
};
pub use message_struct::{EsFluentStructExpansion, EsFluentStructField, EsFluentStructFieldAccess};
pub use variants::{EsFluentGeneratedVariant, EsFluentVariantsExpansion, EsFluentVariantsTarget};

use darling::FromDeriveInput as _;
use es_fluent_shared::{fluent::FluentMessageId, meta::TypeKind, namespace::NamespaceRule};
use heck::ToPascalCase as _;
use syn::Data;

use crate::{
    context::{ContainerContext, ContainerEnvelope},
    error::{AttrContext, AttrError, EsFluentCoreError},
    lowered,
    namespace::{SpannedNamespaceRule, SpannedNamespaceRuleRef},
    options::{
        EnumDataOptions as _, FluentField, GeneratedVariantsOptions, VariantFields as _,
        choice::{CaseStyle, ChoiceOpts},
        r#enum::{EnumOpts, EnumVariantsOpts},
        label::LabelOpts,
        r#struct::{StructOpts, StructVariantsOpts},
    },
    semantic::{
        ArgumentModel, ChoiceModel, ChoiceVariantSource, DerivePathList, GeneratedDocName,
        GeneratedEnumModel, GeneratedKeyIdent, GeneratedKeyName, GeneratedVariantMessageSeed,
        MessageEntryModel, MessageModel, RustSourceName, RustTypeName, SpannedValue,
        generated_label_message_value,
    },
    validation::{self as derive_validation, NamespaceSource, resolve_single_namespace_source},
};

use validation::{validate_container_domain, validate_container_namespace, validate_namespace};

#[cfg(test)]
mod tests {
    include!("expansion/tests.rs");
}
