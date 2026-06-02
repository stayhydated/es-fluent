use es_fluent_derive::EsFluentChoice;

#[derive(EsFluentChoice)]
#[fluent_choice(rename_all = "Title Case")]
pub enum Severity {
    VeryHigh,
}

fn main() {}
