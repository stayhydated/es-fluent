use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
#[fluent(domain = "auth")]
pub struct DomainOnStruct {
    value: String,
}

#[derive(EsFluent)]
#[fluent(resource = "auth_error")]
pub struct ResourceOnStruct {
    value: String,
}

fn main() {}
