pub mod component;
pub mod connection;
pub mod de;
pub mod linker;
pub mod module;
pub mod port;
pub mod prelude;
pub mod reference;
pub mod ser;
pub mod validate;

use ::toml::{de as toml_de, ser as toml_ser};
use serde_json;
use serde_yaml;
use thiserror::Error;

pub use prelude::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"linker error: "{0}""#)]
    Linker(#[from] linker::Error),
    #[error(r#"validation failed: "{0}""#)]
    Validation(#[from] validate::Error),
    #[error(r#"unhandled error occurred: "{0}""#)]
    Generic(Box<dyn std::error::Error>),
}

macro_rules! impl_formats {
    ($($fmt:ident = { ser = $ser:ident, de = $de:ident $(, pretty = $pretty:expr)? $(,)? }),* $(,)?) => {
        $(
            pub mod $fmt {
                use super::prelude::*;

                pub fn from_str(s: &str) -> Result<Module, super::$de::Error> {
                    super::$de::from_str(s)
                }

                pub fn to_string(module: &Module) -> Result<String, super::$ser::Error> {
                    super::$ser::to_string(module)
                }

                $(
                    pub fn to_string_pretty(module: &Module) -> Result<String, super::$ser::Error> {
                        const _: () = assert!($pretty, "format does not support pretty printing!");
                        super::$ser::to_string_pretty(module)
                    }
                )?
            }
        )*
    }
}

impl_formats!(
    json = { ser = serde_json, de = serde_json, pretty = true },
    yaml = { ser = serde_yaml, de = serde_yaml },
    toml = { ser = toml_ser, de = toml_de, pretty = true },
);
