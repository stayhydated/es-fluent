extern crate es_fluent;

use es_fluent_derive::{EsFluentLabel, EsFluentVariants};

#[derive(EsFluentLabel, EsFluentVariants)]
pub enum LoginState {
    #[fluent_variants(skip)]
    Loading,
    Ready,
}

fn main() {}
