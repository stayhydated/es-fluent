//! Tree command for displaying FTL structure.
//!
//! This module provides functionality to display a tree view of FTL items
//! for each FTL file associated with a crate.

mod command;
mod json;
mod links;
mod source_map;
mod text;
mod validation;

pub(crate) use command::{TreeArgs, run_tree};

#[cfg(test)]
mod tests;
