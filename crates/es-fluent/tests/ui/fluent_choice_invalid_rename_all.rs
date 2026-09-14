use es_fluent_derive::EsFluentChoice;

#[derive(EsFluentChoice)]
#[fluent_choice(rename_all = "not_a_style")]
pub enum Severity {
    Low,
    High,
}

fn main() {}
