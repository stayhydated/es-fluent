//! Shared attribute grammar for derive and locale field validation.

mod definitions;
mod rules;
mod validation;

pub use definitions::{
    AttributeFamily, AttributeKey, AttributeLocation, AttributeName, AttributeValueShape,
    FluentAttributeKey,
};
pub use validation::AttributeItem;

pub(crate) use rules::{ATTRIBUTE_RULES, AttributeRule, attribute_rule};
#[cfg(test)]
pub(crate) use validation::help_for_location;
pub(crate) use validation::{parse_attribute_meta_item, validate_attribute_for_family};

#[cfg(test)]
mod tests;
