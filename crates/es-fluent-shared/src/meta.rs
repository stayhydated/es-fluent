//! This module provides types for representing the kind of a type.

use serde::Serialize;

#[derive(
    Clone, Copy, Debug, strum::Display, Eq, Hash, strum::IntoStaticStr, PartialEq, Serialize,
)]
#[strum(const_into_str, serialize_all = "snake_case")]
pub enum TypeKind {
    Enum,
    Struct,
}

impl TypeKind {
    pub const fn label(self) -> &'static str {
        self.into_str()
    }
}

#[cfg(test)]
mod tests {
    use super::TypeKind;

    #[test]
    fn type_kind_labels_use_const_static_str_mapping() {
        const ENUM_LABEL: &str = TypeKind::Enum.label();

        assert_eq!(ENUM_LABEL, "enum");
        assert_eq!(TypeKind::Struct.label(), "struct");
    }
}
