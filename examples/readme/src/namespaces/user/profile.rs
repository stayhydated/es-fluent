use es_fluent::{EsFluent, EsFluentThis};

#[derive(EsFluent)]
#[fluent(namespace(file(relative)))]
pub enum Gender {
    Male,
    Female,
    Other(String),
    Helicopter { type_: String },
}

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace(file(relative)))]
pub struct UserProfile;

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace = folder)]
pub enum FolderStatus {
    Active,
    Inactive,
}

#[derive(EsFluentThis)]
#[fluent_this(origin)]
#[fluent(namespace(folder(relative)))]
pub struct FolderUserProfile;
