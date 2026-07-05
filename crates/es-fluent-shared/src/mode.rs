/// The mode to use when parsing Fluent files.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    strum::Display,
    Eq,
    PartialEq,
    clap::ValueEnum,
    serde::Deserialize,
    serde::Serialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FluentParseMode {
    /// Overwrite existing translations.
    Aggressive,
    /// Preserve existing translations.
    #[default]
    Conservative,
}
