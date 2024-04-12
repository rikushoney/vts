pub mod component;
pub mod connection;
pub mod de;
pub mod linker;
pub mod module;
pub mod port;
pub mod prelude;
pub mod reference;
pub mod ser;

pub use prelude::*;

pub mod json {
    use serde_json::Result;

    use super::prelude::*;

    pub fn from_str(s: &str) -> Result<Module> {
        serde_json::from_str(s)
    }

    pub fn to_string(module: &Module) -> Result<String> {
        serde_json::to_string(module)
    }

    pub fn to_string_pretty(module: &Module) -> Result<String> {
        serde_json::to_string_pretty(module)
    }
}

pub mod yaml {
    use serde_yaml::Result;

    use super::prelude::*;

    pub fn from_str(s: &str) -> Result<Module> {
        serde_yaml::from_str(s)
    }

    pub fn to_string(module: &Module) -> Result<String> {
        serde_yaml::to_string(module)
    }
}

pub mod toml {
    use toml::{de, ser};

    use super::prelude::*;

    pub fn from_str(s: &str) -> Result<Module, de::Error> {
        de::from_str(s)
    }

    pub fn to_string(module: &Module) -> Result<String, ser::Error> {
        ser::to_string(module)
    }

    pub fn to_string_pretty(module: &Module) -> Result<String, ser::Error> {
        ser::to_string_pretty(module)
    }
}
