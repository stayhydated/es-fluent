use es_fluent::{EsFluent, FluentValue, ToFluentString};

#[derive(Clone, EsFluent)]
pub struct GenericStruct<T>
where
    T: Into<FluentValue<'static>> + Clone,
{
    pub field: T,
}

#[derive(Clone, EsFluent)]
pub struct GenericTupleStruct<T>(pub T)
where
    T: Into<FluentValue<'static>> + Clone;

#[derive(Clone, EsFluent)]
pub enum GenericEnum<T>
where
    T: Into<FluentValue<'static>> + Clone,
{
    Variant(T),
    StructVariant { field: T },
    Unit,
}

#[test]
fn test_generics_compilation() {
    let s = GenericStruct {
        field: "hello".to_string(),
    };
    let _ = s.to_fluent_string();

    let ts = GenericTupleStruct(8u8);
    let _ = ts.to_fluent_string();

    let e = GenericEnum::Variant("world".to_string());
    let _ = e.to_fluent_string();
}
