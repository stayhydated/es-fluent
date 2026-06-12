use es_fluent::{EsFluent, EsFluentLabel};

#[derive(EsFluent)]
#[fluent(namespace = file_relative)]
pub enum Gender {
    Male,
    Female,
    Other(String),
    Helicopter { type_: String },
}

#[derive(EsFluentLabel)]
#[fluent(namespace = file_relative)]
pub struct UserProfile;

#[derive(EsFluentLabel)]
#[fluent(namespace = folder)]
pub enum FolderStatus {
    Active,
    Inactive,
}

#[derive(EsFluentLabel)]
#[fluent(namespace = folder_relative)]
pub struct FolderUserProfile;
