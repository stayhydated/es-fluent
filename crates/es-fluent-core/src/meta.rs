//! This module provides types for representing the kind of a type.

use serde::Serialize;

#[derive(Clone, Debug)]
pub struct EnumKind;

#[derive(Clone, Debug)]
pub struct EnumKvKind;

#[derive(Clone, Debug)]
pub struct StructKind;

#[derive(Clone, Debug)]
pub struct StructKvKind;

#[derive(Clone, Debug, strum::Display, Eq, Hash, strum::IntoStaticStr, PartialEq, Serialize)]
#[strum(serialize_all = "snake_case")]
pub enum TypeKind {
    Enum,
    Struct,
}
