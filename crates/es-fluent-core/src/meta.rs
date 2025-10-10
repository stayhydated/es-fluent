//! This module provides types for representing the kind of a type.

use strum::{Display, EnumString};

/// A unit struct that represents a struct kind.
pub struct StructKind;
/// A unit struct that represents an enum kind.
pub struct EnumKind;

/// An enum that represents the kind of a type.
#[derive(Clone, Debug, Display, EnumString, Eq, Hash, PartialEq, serde::Serialize)]
pub enum TypeKind {
    /// A struct.
    Struct,
    /// An enum.
    Enum,
}
