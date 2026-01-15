use es_fluent::EsFluent;

/// A greeting message that requires a name variable.
#[derive(EsFluent)]
pub struct Greeting {
    pub name: String,
}

/// A notification with multiple variables.
#[derive(EsFluent)]
pub struct Notification {
    pub user: String,
    pub count: u32,
}

/// An enum with multiple variants, some missing from FTL.
#[derive(EsFluent)]
pub enum Status {
    Active,
    Inactive,
    /// This variant is missing from the FTL file
    Pending,
}
