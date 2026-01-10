use es_fluent::EsFluent;

/// Test struct with an allow_unused field.
/// The FTL will NOT include $debug_id, but CLI check should pass.
#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
    #[fluent(allow_unused)]
    pub debug_id: u32,
}

/// Test enum demonstrating skip vs allow_unused.
#[derive(EsFluent)]
pub enum Notification<'a> {
    /// Named variant: skip means not passed to Fluent at all
    Alert {
        message: &'a str,
        #[fluent(skip)]
        internal_debug: &'a str,
    },
    /// Tuple variant: allow_unused means passed but optional in FTL
    Info(&'a str, #[fluent(allow_unused)] u32),
}
