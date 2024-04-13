use std::collections::{HashMap, HashSet};

use thiserror::Error;
use ustr::{ustr, Ustr};

use super::{component::ComponentKey, linker::ResolvedComponent, prelude::*, Component};

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"component "{component}" already in "{module}""#)]
    ComponentExists { module: String, component: String },
    #[error(r#"port "{port}" already in "{component}""#)]
    PortExists { component: String, port: String },
    #[error(r#"component "{reference}" already referenced in "{component}""#)]
    ReferenceExists {
        component: String,
        reference: String,
    },
}

impl Error {
    pub fn component_exists(module: &str, component: &str) -> Self {
        Self::ComponentExists {
            module: module.to_string(),
            component: component.to_string(),
        }
    }

    pub fn port_exists(component: &str, port: &str) -> Self {
        Self::PortExists {
            component: component.to_string(),
            port: port.to_string(),
        }
    }

    pub fn reference_exists(component: &str, reference: &str) -> Self {
        Self::ReferenceExists {
            component: component.to_string(),
            reference: reference.to_string(),
        }
    }
}

pub(super) struct CheckComponent {
    #[allow(unused)] // TODO: remove!
    component: ComponentId,
    ports: HashSet<Ustr>,
    references: HashSet<Ustr>,
}

impl CheckComponent {
    pub fn new(component: Component<'_>) -> Self {
        let ports = HashSet::from_iter(component.ports().map(|port| ustr(port.name())));

        let references = HashSet::from_iter(
            component
                .references()
                .map(|reference| ustr(reference.alias_or_name())),
        );

        // TODO: create connection checker

        Self {
            component: component.1,
            ports,
            references,
        }
    }
}

impl From<ResolvedComponent> for CheckComponent {
    fn from(component: ResolvedComponent) -> Self {
        Self {
            component: component.component,
            ports: component.ports,
            references: component.references,
        }
    }
}

pub struct Checker {
    pub(super) components: HashMap<Ustr, CheckComponent>,
}

impl Checker {
    pub fn new(module: &Module) -> Self {
        let components = HashMap::from_iter(
            module
                .components()
                .map(|component| (ustr(component.name()), CheckComponent::new(component))),
        );

        Self { components }
    }

    pub fn ensure_no_existing_component(
        &self,
        module: &Module,
        component: &str,
    ) -> Result<(), Error> {
        if self.components.contains_key(&ustr(component)) {
            Err(Error::component_exists(module.name(), component))
        } else {
            Ok(())
        }
    }

    fn get_checker(&self, module: &Module, component: ComponentId) -> &CheckComponent {
        self.components
            .get(&module[component].name)
            .expect("component should be in checker")
    }

    pub fn ensure_no_existing_port(
        &self,
        module: &Module,
        component: ComponentKey,
        port: &str,
    ) -> Result<(), Error> {
        let component = Component::new(module, component.0);
        let checker = self.get_checker(module, component.1);

        if checker.ports.contains(&ustr(port)) {
            Err(Error::port_exists(component.name(), port))
        } else {
            Ok(())
        }
    }

    pub fn ensure_no_existing_reference(
        &self,
        module: &Module,
        component: ComponentKey,
        reference: &str,
    ) -> Result<(), Error> {
        let component = Component::new(module, component.0);
        let checker = self.get_checker(module, component.1);

        if checker.references.contains(&ustr(reference)) {
            Err(Error::reference_exists(component.name(), reference))
        } else {
            Ok(())
        }
    }

    pub fn ensure_no_colliding_connection(&self) {
        todo!()
    }
}
