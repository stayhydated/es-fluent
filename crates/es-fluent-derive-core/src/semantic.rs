//! Typed semantic values built from parsed derive attributes.

mod arguments;
mod choices;
mod derive_paths;
mod generated;
mod messages;
mod names;

pub use arguments::{ArgumentModel, ArgumentValueStrategy, ValueTransform};
pub use choices::{ChoiceModel, ChoiceVariantModel, ChoiceVariantSource};
pub use derive_paths::{DerivePath, DerivePathList};
pub use generated::{GeneratedEnumModel, GeneratedVariantMessageSeed};
pub use messages::{MessageEntryModel, MessageModel};
pub use names::{
    FluentChoiceValue, GeneratedDocName, GeneratedKeyIdent, GeneratedKeyName, RustSourceName,
    RustTypeName, SourceLocation, SpannedValue, generated_label_message_id,
    generated_label_message_value, generated_variant_message_id, label_message_id_for_ident,
    message_id_for_ident, message_id_from_fluent_key, parse_arg_name, parse_arg_name_in_context,
    parse_domain_name_in_context, parse_fluent_message_id_in_context, parse_variant_key_in_context,
    spanned_message_id_from_value, variant_message_id,
};

pub use es_fluent_shared::fluent::{
    FluentArgumentName as ArgName, FluentDomain as DomainName, FluentMessageId,
    FluentVariantKey as VariantKey,
};

#[cfg(test)]
use crate::{error::AttrContext, options::choice::CaseStyle};
#[cfg(test)]
use es_fluent_shared::{meta::TypeKind, namer};
#[cfg(test)]
use proc_macro2::Span;

#[cfg(test)]
mod tests;
