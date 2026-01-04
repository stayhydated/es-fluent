pub use es_fluent_generate::FluentParseMode;

impl FluentParseModeExt for FluentParseMode {
    /// Returns the string representation for use in generated code.
    fn as_code(&self) -> &'static str {
        match self {
            FluentParseMode::Aggressive => stringify!(FluentParseMode::Aggressive),
            FluentParseMode::Conservative => stringify!(FluentParseMode::Conservative),
        }
    }
}

/// Extension trait for FluentParseMode to add CLI-specific functionality.
pub trait FluentParseModeExt {
    /// Returns the string representation for use in generated code.
    fn as_code(&self) -> &'static str;
}
