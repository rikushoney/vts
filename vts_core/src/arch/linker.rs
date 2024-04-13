use std::collections::{HashMap, HashSet};

use thiserror::Error;
use ustr::{ustr, Ustr};

use super::{
    checker::{CheckComponent, Checker},
    component::ComponentKey,
    connection::WeakConnection,
    module::ComponentId,
    prelude::*,
    reference::ComponentWeakRef,
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

pub struct KnownComponents(pub(super) HashMap<Ustr, ComponentId>);

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
        self.0.get(&ustr(component)).copied().map(make_component)
    }
}

pub(super) struct ResolvedComponent {
    pub(super) component: ComponentId,
    pub(super) ports: HashSet<Ustr>,
    pub(super) references: HashSet<Ustr>,
}

pub struct ResolvedComponents(pub(super) HashMap<Ustr, ResolvedComponent>);

impl ResolvedComponents {
    pub fn into_checker(self, module: &Module) -> Checker {
        let mut checker = Checker::new(module);

        checker.components = HashMap::from_iter(
            self.0
                .into_iter()
                .map(|(component, resolved)| (component, CheckComponent::from(resolved))),
        );

        checker
    }
}

#[derive(Default)]
pub struct Linker {
    unresolved: HashMap<ComponentId, LinkerItems>,
}

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
        KnownComponents(HashMap::from_iter(
            module
                .components
                .iter()
                .map(|(component, data)| (data.name.clone(), component)),
        ))
    }

    fn get_known_ports(module: &Module, component: ComponentId) -> HashSet<Ustr> {
        let component = Component::new(module, component);
        HashSet::from_iter(component.ports().map(|port| ustr(port.name())))
    }

    fn resolve_impl(
        module: &mut Module,
        component: ComponentId,
        unresolved: &mut LinkerItems,
        components: &KnownComponents,
    ) -> Result<ResolvedComponent, Error> {
        let references = unresolved.references.drain(..).try_fold(
            HashSet::default(),
            |mut resolved, reference| {
                let reference =
                    reference.resolve(module, ComponentKey::new(component), components)?;

                resolved.insert(ustr(ComponentRef::new(module, reference.0).alias_or_name()));
                Ok(resolved)
            },
        )?;

        unresolved
            .connections
            .drain(..)
            .try_for_each(|connection| {
                connection.resolve(module, ComponentKey::new(component), components)?;
                Ok(())
            })?;

        Ok(ResolvedComponent {
            component,
            ports: Self::get_known_ports(module, component),
            references,
        })
    }

    pub fn resolve(&mut self, module: &mut Module) -> Result<ResolvedComponents, Error> {
        let components = Self::get_known_components(module);

        let resolved = self.unresolved.iter_mut().try_fold(
            HashMap::default(),
            |mut resolved, (&component, unresolved)| {
                resolved.insert(
                    module[component].name,
                    Self::resolve_impl(module, component, unresolved, &components)?,
                );

                Ok::<_, Error>(resolved)
            },
        )?;

        Ok(ResolvedComponents(resolved))
    }
}
