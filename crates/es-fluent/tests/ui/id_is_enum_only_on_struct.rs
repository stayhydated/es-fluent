use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
#[fluent(id = "auth_error")]
pub struct IdOnStruct {
    value: String,
}

fn main() {}
