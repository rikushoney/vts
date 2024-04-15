use std::collections::{HashMap, HashSet};

use thiserror::Error;
use ustr::{ustr, Ustr};

use super::{
    checker::{self, CheckComponent, Checker},
    component::ComponentKey,
    connection::WeakConnection,
    module::ComponentId,
    port::PortKey,
    prelude::*,
    reference::ComponentWeakRef,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"{0}"#)]
    Checker(#[from] checker::Error),
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

pub trait Resolve<'a, 'm> {
    type Output;

    fn resolve(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
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
    checker: Checker,
    unresolved: HashMap<ComponentId, LinkerItems>,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            checker: Checker::default(),
            unresolved: HashMap::default(),
        }
    }

    pub fn checker(&self) -> &Checker {
        &self.checker
    }

    pub fn checker_mut(&mut self) -> &mut Checker {
        &mut self.checker
    }

    pub fn register_component(
        &mut self,
        module: &Module,
        component: ComponentKey,
    ) -> Result<(), Error> {
        Ok(self.checker.register_component(module, component)?)
    }

    pub fn register_port(
        &mut self,
        module: &Module,
        component: ComponentKey,
        port: PortKey,
    ) -> Result<(), Error> {
        Ok(self.checker.register_port(module, component, port)?)
    }

    pub fn register_reference(
        &mut self,
        module: &Module,
        component: ComponentKey,
        reference: ComponentWeakRef,
    ) -> Result<(), Error> {
        self.checker
            .ensure_no_existing_reference(module, component, reference.alias_or_name())?;

        self.unresolved
            .entry(component.0)
            .or_default()
            .references
            .push(reference);

        Ok(())
    }

    pub fn register_connection(
        &mut self,
        component: ComponentKey,
        connection: WeakConnection,
    ) -> Result<(), Error> {
        self.checker_mut().register_connection()?;

        self.unresolved
            .entry(component.0)
            .or_default()
            .connections
            .push(connection);

        Ok(())
    }

    fn get_known_components(module: &Module) -> KnownComponents {
        KnownComponents(HashMap::from_iter(
            module
                .components
                .iter()
                .map(|(component, data)| (data.name, component)),
        ))
    }

    fn get_known_ports(module: &Module, component: ComponentId) -> HashSet<Ustr> {
        let component = Component::new(module, component);
        HashSet::from_iter(component.ports().map(|port| ustr(port.name())))
    }

    fn resolve_impl(
        module: &mut Module,
        checker: &mut Checker,
        component: ComponentId,
        unresolved: &mut LinkerItems,
        components: &KnownComponents,
    ) -> Result<ResolvedComponent, Error> {
        let references = unresolved.references.drain(..).try_fold(
            HashSet::default(),
            |mut resolved, reference| {
                let reference =
                    reference.resolve(module, checker, ComponentKey::new(component), components)?;

                resolved.insert(ustr(ComponentRef::new(module, reference.0).alias_or_name()));
                Ok::<_, Error>(resolved)
            },
        )?;

        unresolved
            .connections
            .drain(..)
            .try_for_each(|connection| {
                connection.resolve(module, checker, ComponentKey::new(component), components)?;
                checker.register_connection()?;
                Ok::<_, Error>(())
            })?;

        Ok(ResolvedComponent {
            component,
            ports: Self::get_known_ports(module, component),
            references,
        })
    }

    pub fn resolve(&mut self, module: &mut Module) -> Result<ResolvedComponents, Error> {
        let components = Self::get_known_components(module);

        let resolved = self.unresolved.drain().try_fold(
            HashMap::default(),
            |mut resolved, (component, mut unresolved)| {
                let reference = Self::resolve_impl(
                    module,
                    &mut self.checker,
                    component,
                    &mut unresolved,
                    &components,
                )?;

                resolved.insert(module[component].name, reference);
                Ok::<_, Error>(resolved)
            },
        )?;

        Ok(ResolvedComponents(resolved))
    }
}
