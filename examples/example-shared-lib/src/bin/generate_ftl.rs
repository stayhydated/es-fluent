// Import everything from the lib crate to ensure all types are linked
// (inventory only collects types that are actually linked into the binary)
use es_fluent::EsFluentGenerator;
#[allow(unused_imports)]
use example_shared_lib::*;

fn main() {
    EsFluentGenerator::builder()
        .build()
        .generate()
        .expect("Failed to generate FTL files");
}
