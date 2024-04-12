use std::collections::HashMap;

use fnv::FnvHashMap;
use thiserror::Error;

use super::{
    component::ComponentKey,
    connection::WeakConnection,
    module::{ComponentId, Module},
    reference::ComponentWeakRef,
    Component,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"undefined component "{component}" referenced in "{module}""#)]
    UndefinedComponent { module: String, component: String },
    #[error(r#"undefined port "{port}" referenced in "{component}""#)]
    UndefinedPort { component: String, port: String },
    #[error(r#"undefined reference "{reference}" referenced in "{component}""#)]
    UndefinedReference {
        component: String,
        reference: String,
    },
}

impl Error {
    pub fn undefined_component(module: &str, component: &str) -> Self {
        Self::UndefinedComponent {
            module: module.to_string(),
            component: component.to_string(),
        }
    }

    pub fn undefined_port(component: &str, port: &str) -> Self {
        Self::UndefinedPort {
            component: component.to_string(),
            port: port.to_string(),
        }
    }

    pub fn undefined_reference(component: &str, alias_or_name: &str) -> Self {
        Self::UndefinedReference {
            component: component.to_string(),
            reference: alias_or_name.to_string(),
        }
    }
}

pub struct KnownComponents(pub(super) FnvHashMap<String, ComponentId>);

pub trait Resolve<'m> {
    type Output;

    fn resolve(
        self,
        module: &'m mut Module,
        parent: ComponentKey,
        components: &KnownComponents,
    ) -> Result<Self::Output, Error>;
}

#[derive(Default)]
struct LinkerItems {
    references: Vec<ComponentWeakRef>,
    connections: Vec<WeakConnection>,
}

impl KnownComponents {
    pub fn get<'m>(&self, module: &'m Module, component: &str) -> Result<Component<'m>, Error> {
        self.try_get(module, component)
            .ok_or(Error::undefined_component(&module.name, component))
    }

    pub fn try_get<'m>(&self, module: &'m Module, component: &str) -> Option<Component<'m>> {
        let make_component = |component| Component::new(module, component);
        self.0.get(component).copied().map(make_component)
    }
}

#[derive(Default)]
pub struct Linker {
    unresolved: HashMap<ComponentId, LinkerItems>,
}

fn discard<T>(_: T) {}

impl Linker {
    pub fn new() -> Self {
        Self {
            unresolved: HashMap::default(),
        }
    }

    pub fn register_reference(&mut self, component: ComponentKey, reference: ComponentWeakRef) {
        // TODO: check duplicate references
        self.unresolved
            .entry(component.0)
            .or_default()
            .references
            .push(reference);
    }

    pub fn register_connection(&mut self, component: ComponentKey, connection: WeakConnection) {
        // TODO: check colliding connections
        self.unresolved
            .entry(component.0)
            .or_default()
            .connections
            .push(connection);
    }

    fn get_known_components(module: &Module) -> KnownComponents {
        KnownComponents(FnvHashMap::from_iter(
            module
                .components
                .iter()
                .map(|(component, data)| (data.name.clone(), component)),
        ))
    }

    fn resolve_impl(
        module: &mut Module,
        component: ComponentId,
        unresolved: &mut LinkerItems,
        components: &KnownComponents,
    ) -> Result<(), Error> {
        unresolved.references.drain(..).try_for_each(|reference| {
            reference
                .resolve(module, ComponentKey::new(component), components)
                .map(discard)
        })?;

        unresolved.connections.drain(..).try_for_each(|connection| {
            connection
                .resolve(module, ComponentKey::new(component), components)
                .map(discard)
        })
    }

    pub fn resolve(&mut self, module: &mut Module) -> Result<(), Error> {
        let components = Self::get_known_components(module);

        self.unresolved
            .iter_mut()
            .try_for_each(|(&component, unresolved)| {
                Self::resolve_impl(module, component, unresolved, &components)
            })
    }
}
