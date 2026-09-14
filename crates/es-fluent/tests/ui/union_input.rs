use es_fluent_derive::{EsFluent, EsFluentVariants};

#[derive(EsFluent)]
pub union MessageUnion {
    a: u32,
    b: f32,
}

#[derive(EsFluentVariants)]
pub union VariantsUnion {
    a: u32,
    b: f32,
}

fn main() {}
