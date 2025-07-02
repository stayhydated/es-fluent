use strum::{Display, EnumString};

pub struct StructKind;
pub struct EnumKind;

#[derive(Clone, Debug, Display, EnumString, Eq, Hash, PartialEq)]
pub enum TypeKind {
    Struct,
    Enum,
}
