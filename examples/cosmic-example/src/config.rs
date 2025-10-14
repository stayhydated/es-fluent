use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};

#[derive(Clone, CosmicConfigEntry, Debug, Default, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    demo: String,
}
