use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
pub struct Invalid {
    #[fluent(skip, arg = "value")]
    value: i32,
}

fn main() {}
