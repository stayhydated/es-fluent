use es_fluent::{EsFluentThis, ThisFtl};

#[derive(EsFluentThis)]
#[fluent_this(origin)]
enum GenderThisOnly {
    Male,
}

fn main() {
    let _ = GenderThisOnly::this_ftl();
}
