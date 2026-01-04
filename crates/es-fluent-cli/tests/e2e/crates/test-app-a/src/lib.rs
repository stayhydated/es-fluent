use es_fluent::{EsFluent, EsFluentChoice};

#[derive(EsFluent, EsFluentChoice)]
pub enum Gender {
    Male,
    Female,
    Other,
}

#[derive(EsFluent)]
pub enum Shared<'a> {
    Photos {
        user_name: &'a str,
        photo_count: &'a u32,
        // this signals the macro to use the choice representation, since we'll
        // match against it in the ftl resource
        #[fluent(choice)]
        user_gender: &'a Gender,
    },
}

#[derive(EsFluent)]
pub struct HelloA;
