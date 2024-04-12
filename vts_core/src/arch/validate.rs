use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"multiple definitions for component "{component}" in "{module}""#)]
    DuplicateComponent { module: String, component: String },
    #[error(r#"multiple definitions for port "{port}" in "{component}""#)]
    DuplicatePort { component: String, port: String },
    #[error(r#"multiple definitions for reference "{reference}" in "{component}""#)]
    DuplicateReference {
        component: String,
        reference: String,
    },
}

impl Error {
    pub fn duplicate_component(module: &str, component: &str) -> Self {
        Self::DuplicateComponent {
            module: module.to_string(),
            component: component.to_string(),
        }
    }

    pub fn duplicate_port(component: &str, port: &str) -> Self {
        Self::DuplicatePort {
            component: component.to_string(),
            port: port.to_string(),
        }
    }

    pub fn duplicate_reference(component: &str, reference: &str) -> Self {
        Self::DuplicateReference {
            component: component.to_string(),
            reference: reference.to_string(),
        }
    }
}
