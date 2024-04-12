use std::slice;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::prelude::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentData {
    pub name: String,
    pub(crate) ports: Vec<PortId>,
    pub(crate) references: Vec<ComponentRefId>,
    pub(crate) connections: Vec<Connection>,
    pub class: Option<ComponentClass>,
}

impl ComponentData {
    fn new(name: &str, class: Option<ComponentClass>) -> Self {
        Self {
            name: name.to_string(),
            ports: Vec::new(),
            references: Vec::new(),
            connections: Vec::new(),
            class,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentKey(pub(crate) ComponentId);

impl ComponentKey {
    pub(crate) fn new(component: ComponentId) -> Self {
        Self(component)
    }

    pub fn bind(self, module: &Module) -> Component<'_> {
        Component::new(module, self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Component<'m>(&'m Module, pub(super) ComponentId);

impl<'m> Component<'m> {
    pub(crate) fn new(module: &'m Module, component: ComponentId) -> Self {
        Self(module, component)
    }

    pub fn module(&self) -> &'m Module {
        self.0
    }

    pub fn key(&self) -> ComponentKey {
        ComponentKey::new(self.1)
    }

    pub(crate) fn data(&self) -> &'m ComponentData {
        &self.module()[self.1]
    }

    pub fn name(&self) -> &'m str {
        &self.data().name
    }

    pub fn ports(&self) -> PortIter<'m> {
        PortIter {
            module: self.module(),
            iter: self.data().ports.iter(),
        }
    }

    pub fn references(&self) -> ComponentRefIter<'m> {
        ComponentRefIter {
            module: self.module(),
            iter: self.data().references.iter(),
        }
    }

    pub fn connections(&self) -> ConnectionIter<'m> {
        ConnectionIter {
            iter: self.data().connections.iter(),
        }
    }

    pub fn class(&self) -> Option<ComponentClass> {
        self.data().class
    }

    pub fn find_port(&self, name: &str) -> Option<Port<'_>> {
        self.data().ports.iter().find_map(|&port| {
            let port = Port::new(self.module(), port);
            if port.name() == name {
                Some(port)
            } else {
                None
            }
        })
    }

    pub fn find_reference(&self, alias_or_name: &str) -> Option<ComponentRef<'_>> {
        self.data().references.iter().find_map(|&reference| {
            let reference = ComponentRef::new(self.module(), reference);
            if reference.alias_or_name() == alias_or_name {
                Some(reference)
            } else {
                None
            }
        })
    }
}

pub struct PortIter<'m> {
    module: &'m Module,
    iter: slice::Iter<'m, PortId>,
}

impl<'m> Iterator for PortIter<'m> {
    type Item = Port<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|&port| Port::new(self.module, port))
    }
}

pub struct ComponentRefIter<'m> {
    module: &'m Module,
    iter: slice::Iter<'m, ComponentRefId>,
}

impl<'m> Iterator for ComponentRefIter<'m> {
    type Item = ComponentRef<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|&reference| ComponentRef::new(self.module, reference))
    }
}

pub struct ConnectionIter<'m> {
    iter: slice::Iter<'m, Connection>,
}

impl<'m> Iterator for ConnectionIter<'m> {
    type Item = &'m Connection;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct NameSet(String);
pub struct NameUnset;

pub struct ComponentBuilder<'m, N> {
    pub(crate) module: &'m mut Module,
    name: N,
    class: Option<ComponentClass>,
}

impl<'m> ComponentBuilder<'m, NameUnset> {
    pub fn new(module: &'m mut Module) -> Self {
        Self {
            module,
            name: NameUnset,
            class: None,
        }
    }

    pub fn set_name(self, name: &str) -> ComponentBuilder<'m, NameSet> {
        ComponentBuilder {
            module: self.module,
            name: NameSet(name.to_string()),
            class: self.class,
        }
    }
}

impl<'m, N> ComponentBuilder<'m, N> {
    pub fn set_class(&mut self, class: ComponentClass) {
        self.class = Some(class)
    }

    pub fn class_is_set(&self) -> bool {
        self.class.is_some()
    }
}

impl<'m> ComponentBuilder<'m, NameSet> {
    fn insert(&mut self) -> ComponentId {
        // TODO: check duplicate components
        let component = ComponentData::new(&self.name.0, self.class);
        self.module.components.insert(component)
    }

    pub fn finish(mut self) -> Component<'m> {
        let component = self.insert();
        Component::new(self.module, component)
    }
}

#[derive(Debug, Error)]
pub enum ComponentBuildError {
    #[error(r#"component "{component}" already in "{module}""#)]
    DuplicateComponent { module: String, component: String },
    #[error(r#"component "{reference}" already referenced in "{component}""#)]
    DuplicateReference {
        component: String,
        reference: String,
    },
    #[error("component must have a {0}")]
    MissingField(&'static str),
}

pub(super) const FIELDS: &[&str] = &[
    "ports",
    "references",
    "named_references",
    "connections",
    "class",
];

pub(super) const PORTS: usize = 0;
pub(super) const REFERENCES: usize = 1;
pub(super) const NAMED_REFERENCES: usize = 2;
pub(super) const CONNECTIONS: usize = 3;
pub(super) const CLASS: usize = 4;
