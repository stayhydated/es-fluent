use crate::error::FluentScParserError;
use bon::Builder;
use darling::FromDeriveInput as _;
use es_fluent_core::analysis;
use es_fluent_core::meta::{EnumKind, StructKind};
use es_fluent_core::options::{r#enum::EnumOpts, r#struct::StructOpts};
use es_fluent_core::registry::FtlTypeInfo;
use log::debug;
use syn::DeriveInput;

pub trait FluentProcessKind {
    fn process(&self, input: &DeriveInput) -> Result<Vec<FtlTypeInfo>, FluentScParserError>;
}

#[derive(Builder, Clone, Debug)]
pub struct FtlProcessor<K> {
    current_file: std::path::PathBuf,
    #[builder(default)]
    _marker: std::marker::PhantomData<K>,
}

impl FluentProcessKind for FtlProcessor<EnumKind> {
    fn process(&self, input: &DeriveInput) -> Result<Vec<FtlTypeInfo>, FluentScParserError> {
        debug!("Processing Enum: {}", input.ident);

        let enum_opts = match EnumOpts::from_derive_input(input) {
            Ok(opts) => opts,
            Err(e) => {
                return Err(FluentScParserError::AttributeParse(
                    self.current_file.clone(),
                    e,
                ));
            },
        };

        Ok(analysis::analyze_enum(&enum_opts))
    }
}

impl FluentProcessKind for FtlProcessor<StructKind> {
    fn process(&self, input: &DeriveInput) -> Result<Vec<FtlTypeInfo>, FluentScParserError> {
        debug!("Processing Struct: {}", input.ident);

        let struct_opts = match StructOpts::from_derive_input(input) {
            Ok(opts) => opts,
            Err(e) => {
                return Err(FluentScParserError::AttributeParse(
                    self.current_file.clone(),
                    e,
                ));
            },
        };

        Ok(analysis::analyze_struct(&struct_opts))
    }
}
