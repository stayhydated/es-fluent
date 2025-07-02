use crate::meta::TypeKind;
use crate::namer::FluentKey;
use bon::Builder;

#[derive(Builder, Clone, Debug, Eq, Hash, PartialEq)]
pub struct FtlVariant {
    pub name: String,
    pub ftl_key: FluentKey,
    pub arguments: Option<Vec<String>>,
}

#[derive(Builder, Clone, Debug, Eq, Hash, PartialEq)]
pub struct FtlTypeInfo {
    pub type_kind: TypeKind,
    pub type_name: String,
    pub variants: Vec<FtlVariant>,
}
