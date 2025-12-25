use es_fluent::EsFluentGenerator;

fn main() {
    EsFluentGenerator::builder()
        .build()
        .generate()
        .expect("Failed to generate FTL files");
}
