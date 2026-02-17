#![doc = include_str!("../README.md")]

pub mod commands;
pub mod core;
pub mod ftl;
pub mod generation;
pub mod tui;
pub mod utils;

#[cfg(test)]
pub(crate) mod test_fixtures;
