use llhd::{assembly, ir::prelude::*};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"parsing error occurred: "{0}""#)]
    Parser(String),
}

impl Error {
    pub fn parser(message: String) -> Self {
        Self::Parser(message)
    }
}

pub fn from_str(source: &str) -> Result<Module, Error> {
    assembly::parse_module(source).map_err(Error::parser)
}
