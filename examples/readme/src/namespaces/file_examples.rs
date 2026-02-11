use es_fluent::{EsFluent, EsFluentThis, EsFluentVariants};

#[derive(EsFluent)]
#[fluent(namespace = file)]
pub struct Dialog {
    pub title: String,
}

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = file)]
pub enum Status {
    Active,
    Inactive,
}

#[derive(EsFluentVariants)]
#[fluent(namespace = file)]
pub enum StatusVariants {
    Active,
    Inactive,
}
