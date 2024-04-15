use std::collections::{HashMap, HashSet};

use thiserror::Error;
use ustr::{ustr, Ustr};

use super::{
    component::ComponentKey, linker::ResolvedComponent, port::PortKey, prelude::*,
    reference::ComponentRefKey, Component,
};

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
    pub fn new(module: &Module, component: ComponentKey) -> Self {
        let component = Component::new(module, component.0);
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

#[derive(Default)]
pub struct Checker {
    pub(super) components: HashMap<Ustr, CheckComponent>,
}

impl Checker {
    pub fn new(module: &Module) -> Self {
        let components = HashMap::from_iter(module.components().map(|component| {
            (
                ustr(component.name()),
                CheckComponent::new(module, component.key()),
            )
        }));

        Self { components }
    }

    fn get_component_checker(&self, module: &Module, component: ComponentId) -> &CheckComponent {
        self.components
            .get(&module[component].name)
            .expect("component should be in checker")
    }

    fn get_component_checker_mut(
        &mut self,
        module: &Module,
        component: ComponentId,
    ) -> &mut CheckComponent {
        self.components
            .get_mut(&module[component].name)
            .expect("component should be in checker")
    }

    pub fn register_component(
        &mut self,
        module: &Module,
        component: ComponentKey,
    ) -> Result<(), Error> {
        let name = module[component.0].name;
        let checker = CheckComponent::new(module, component);

        if self.components.insert(name, checker).is_none() {
            Ok(())
        } else {
            Err(Error::component_exists(module.name(), &name))
        }
    }

    pub fn register_port(
        &mut self,
        module: &Module,
        component: ComponentKey,
        port: PortKey,
    ) -> Result<(), Error> {
        let name = module[port.0].name;
        let checker = self.get_component_checker_mut(module, component.0);

        if checker.ports.insert(name) {
            Ok(())
        } else {
            let component = module[component.0].name;
            Err(Error::port_exists(&component, &name))
        }
    }

    pub fn register_reference(
        &mut self,
        module: &Module,
        component: ComponentKey,
        reference: ComponentRefKey,
    ) -> Result<(), Error> {
        let reference = ustr(ComponentRef::new(module, reference.0).alias_or_name());
        let checker = self.get_component_checker_mut(module, component.0);

        if checker.references.insert(reference) {
            Ok(())
        } else {
            let component = module[component.0].name;
            Err(Error::reference_exists(&component, &reference))
        }
    }

    pub fn register_connection(&mut self) -> Result<(), Error> {
        // TODO:
        Ok(())
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

    pub fn ensure_no_existing_port(
        &self,
        module: &Module,
        component: ComponentKey,
        port: &str,
    ) -> Result<(), Error> {
        let component = Component::new(module, component.0);
        let checker = self.get_component_checker(module, component.1);

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
        let checker = self.get_component_checker(module, component.1);

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
