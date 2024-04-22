use std::slice;

use serde::{Deserialize, Serialize};
use ustr::{ustr, Ustr};

use super::{checker, prelude::*};

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

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComponentData {
    pub name: Ustr,
    pub ports: Vec<PortId>,
    pub references: Vec<ComponentRefId>,
    pub connections: Vec<ConnectionId>,
    pub class: Option<ComponentClass>,
}

impl ComponentData {
    fn new(name: &str, class: Option<ComponentClass>) -> Self {
        Self {
            name: ustr(name),
            ports: Vec::new(),
            references: Vec::new(),
            connections: Vec::new(),
            class,
        }
    }
}

mod component_access {
    use super::*;

    pub trait Sealed {}

    impl Sealed for ComponentId {}

    impl Sealed for Component<'_> {}
}

pub trait ComponentAccess: Copy + component_access::Sealed {
    fn id(&self) -> ComponentId;
    fn bind<'m>(&self, module: &'m Module) -> Component<'m>;
}

impl ComponentAccess for ComponentId {
    fn id(&self) -> ComponentId {
        *self
    }

    fn bind<'m>(&self, module: &'m Module) -> Component<'m> {
        Component::new(module, *self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Component<'m>(&'m Module, ComponentId);

impl<'m> Component<'m> {
    fn new(module: &'m Module, component: ComponentId) -> Self {
        Self(module, component)
    }

    pub fn module(&self) -> &'m Module {
        self.0
    }

    pub fn unbind(self) -> ComponentId {
        self.1
    }

    pub(crate) fn data(&self) -> &'m ComponentData {
        self.module().lookup(self.1)
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
            module: self.module(),
            iter: self.data().connections.iter(),
        }
    }

    pub fn class(&self) -> Option<ComponentClass> {
        self.data().class
    }

    pub fn find_port(&self, name: &str) -> Option<Port<'m>> {
        self.data().ports.iter().find_map(|port| {
            let port = port.bind(self.module());
            (port.name() == name).then_some(port)
        })
    }

    pub fn find_reference(&self, alias_or_name: &str) -> Option<ComponentRef<'_>> {
        self.data().references.iter().find_map(|reference| {
            let reference = reference.bind(self.module());
            (reference.alias_or_name() == alias_or_name).then_some(reference)
        })
    }
}

impl ComponentAccess for Component<'_> {
    fn id(&self) -> ComponentId {
        self.1
    }

    fn bind<'m>(&self, module: &'m Module) -> Component<'m> {
        self.1.bind(module)
    }
}

pub struct PortIter<'m> {
    module: &'m Module,
    iter: slice::Iter<'m, PortId>,
}

impl<'m> Iterator for PortIter<'m> {
    type Item = Port<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|port| port.bind(self.module))
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
            .map(|reference| reference.bind(self.module))
    }
}

pub struct ConnectionIter<'m> {
    module: &'m Module,
    iter: slice::Iter<'m, ConnectionId>,
}

impl<'m> Iterator for ConnectionIter<'m> {
    type Item = Connection<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|connection| connection.bind(self.module))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct NameSet(String);
pub struct NameUnset;

pub struct ComponentBuilder<'a, 'm, N> {
    module: &'m mut Module,
    checker: &'a mut Checker,
    name: N,
    class: Option<ComponentClass>,
}

impl<'a, 'm> ComponentBuilder<'a, 'm, NameUnset> {
    pub fn new(module: &'m mut Module, checker: &'a mut Checker) -> Self {
        Self {
            module,
            checker,
            name: NameUnset,
            class: None,
        }
    }

    pub fn set_name(self, name: &str) -> ComponentBuilder<'a, 'm, NameSet> {
        ComponentBuilder {
            module: self.module,
            checker: self.checker,
            name: NameSet(name.to_string()),
            class: self.class,
        }
    }
}

impl<'m, N> ComponentBuilder<'_, 'm, N> {
    pub fn set_class(&mut self, class: ComponentClass) {
        self.class = Some(class)
    }

    pub fn class_is_set(&self) -> bool {
        self.class.is_some()
    }
}

impl<'m> ComponentBuilder<'_, 'm, NameSet> {
    fn insert(&mut self) -> ComponentId {
        let component = ComponentData::new(&self.name.0, self.class);
        self.module.components.insert(component)
    }

    pub fn finish(mut self) -> Result<Component<'m>, checker::Error> {
        let component = self.insert();
        self.checker.register_component(self.module, component)?;
        Ok(Component::new(self.module, component))
    }
}
