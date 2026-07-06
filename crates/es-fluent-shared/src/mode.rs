/// The mode to use when parsing Fluent files.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    strum::Display,
    strum::IntoStaticStr,
    Eq,
    PartialEq,
    clap::ValueEnum,
    serde::Deserialize,
    serde::Serialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(const_into_str, serialize_all = "snake_case")]
pub enum FluentParseMode {
    /// Overwrite existing translations.
    Aggressive,
    /// Preserve existing translations.
    #[default]
    Conservative,
}

impl FluentParseMode {
    pub const fn label(self) -> &'static str {
        self.into_str()
    }
}

#[cfg(test)]
mod tests {
    use super::FluentParseMode;

    #[test]
    fn fluent_parse_mode_labels_use_const_static_str_mapping() {
        const CONSERVATIVE_LABEL: &str = FluentParseMode::Conservative.label();

        assert_eq!(CONSERVATIVE_LABEL, "conservative");
        assert_eq!(FluentParseMode::Aggressive.label(), "aggressive");
    }
}
