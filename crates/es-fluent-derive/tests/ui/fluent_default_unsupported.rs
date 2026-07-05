use es_fluent_derive::EsFluent;

#[derive(EsFluent)]
pub struct LoginForm {
    #[fluent(default)]
    username: String,
}

fn main() {}
