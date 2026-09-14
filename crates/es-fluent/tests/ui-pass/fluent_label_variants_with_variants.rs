extern crate es_fluent;

use es_fluent_derive::{EsFluentLabel, EsFluentVariants};

#[derive(EsFluentLabel, EsFluentVariants)]
pub struct LoginForm<'a, T>
where
    T: 'a,
{
    username: &'a T,
}

fn main() {}
