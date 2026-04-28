fn main() {
    es_fluent_manager_embedded::init();
    es_fluent_manager_embedded::try_init();
    es_fluent_manager_embedded::init_with_language("en-US");
    es_fluent_manager_embedded::try_init_with_language("en-US");
}
