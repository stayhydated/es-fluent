//! This module provides types for representing the kind of a type.

use strum::{Display, EnumString};

pub struct StructKind;

pub struct EnumKind;

#[derive(Clone, Debug, Display, EnumString, Eq, Hash, PartialEq, serde::Serialize)]
pub enum TypeKind {
    Struct,

    Enum,
}
