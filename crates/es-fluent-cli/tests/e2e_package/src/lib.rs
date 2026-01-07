use es_fluent::EsFluent;

#[derive(EsFluent)]
#[fluent(resource = "test-app-package.ftl")]
pub enum Dictionary {
    #[fluent(key = "hello")]
    Hello,
}
