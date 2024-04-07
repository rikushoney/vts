pub mod de;
pub mod ser;

use std::collections::{hash_map, HashMap, HashSet};
use std::ops::{Index, Range};
use std::slice;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::arch::{
    port::{Port, PortBuilder, PortData, PortPins, WeakPortPins},
    Module, PortId, StringId,
};

impl_dbkey_wrapper!(ComponentId, u32);

impl ComponentId {
    pub fn to_component(self, module: &Module) -> Component<'_> {
        Component::new(module, self)
    }
}

impl_dbkey_wrapper!(ComponentRefId, u32);

impl ComponentRefId {
    pub fn to_component(self, module: &Module) -> Component<'_> {
        let reference = &module[self];
        Component::new(module, reference.component)
    }

    pub fn to_reference(self, module: &Module) -> ComponentRef<'_> {
        ComponentRef::new(module, self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentData {
    pub(crate) name: StringId,
    pub(crate) ports: HashMap<StringId, PortId>,
    pub(crate) references: HashMap<StringId, ComponentRefId>,
    pub(crate) connections: Vec<Connection>,
    pub class: Option<ComponentClass>,
}

impl ComponentData {
    fn new(module: &mut Module, name: &str, class: Option<ComponentClass>) -> Self {
        let name = module.strings.entry(name);
        assert!(
            module.components.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = &module.strings[name],
            module = &module.strings[module.name]
        );

        let ports = HashMap::default();
        let references = HashMap::default();
        let connections = Vec::new();

        Self {
            name,
            ports,
            references,
            connections,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        &module.strings[self.name]
    }

    pub fn get_port<'m>(&self, module: &'m Module, port: PortId) -> Option<&'m PortData> {
        if self.ports.values().any(|p| p == &port) {
            Some(&module[port])
        } else {
            None
        }
    }

    pub fn get_port_mut<'m>(
        &'m self,
        module: &'m mut Module,
        port: PortId,
    ) -> Option<&'m mut PortData> {
        if self.ports.values().any(|p| p == &port) {
            Some(&mut module[port])
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRefData {
    pub(crate) component: ComponentId,
    alias: StringId,
    pub n_instances: usize,
}

impl ComponentRefData {
    pub(crate) fn new(component: ComponentId, alias: StringId, n_instances: usize) -> Self {
        Self {
            component,
            alias,
            n_instances,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component<'m> {
    module: &'m Module,
    id: ComponentId,
    data: &'m ComponentData,
}

impl<'m> Component<'m> {
    fn new(module: &'m Module, id: ComponentId) -> Self {
        let data = &module.component_db[id];

        Self { module, id, data }
    }

    pub fn name(&self) -> &'m str {
        self.data.name(self.module)
    }

    pub fn ports(&self) -> PortIter<'m> {
        PortIter {
            module: self.module,
            iter: self.data.ports.values(),
        }
    }

    pub fn references(&self) -> ComponentRefIter<'m> {
        ComponentRefIter {
            module: self.module,
            iter: self.data.references.iter(),
        }
    }

    pub fn connections(&self) -> ConnectionIter<'m> {
        ConnectionIter {
            iter: self.data.connections.iter(),
        }
    }

    pub fn class(&self) -> Option<ComponentClass> {
        self.data.class
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRef<'m> {
    module: &'m Module,
    id: ComponentRefId,
    data: &'m ComponentRefData,
}

impl<'m> ComponentRef<'m> {
    fn new(module: &'m Module, id: ComponentRefId) -> Self {
        let data = &module[id];

        Self { module, id, data }
    }

    pub fn component(&self) -> Component<'m> {
        Component::new(self.module, self.data.component)
    }

    pub fn alias(&self) -> &'m str {
        &self.module.strings[self.data.alias]
    }

    pub fn n_instances(&self) -> usize {
        self.data.n_instances
    }
}

impl<'m> PartialEq<ComponentRef<'m>> for &Component<'m> {
    fn eq(&self, other: &ComponentRef) -> bool {
        self.id == other.data.component
    }
}

impl<'m> Index<PortId> for Component<'m> {
    type Output = PortData;

    fn index(&self, port: PortId) -> &Self::Output {
        &self.module[port]
    }
}

pub struct PortIter<'m> {
    module: &'m Module,
    iter: hash_map::Values<'m, StringId, PortId>,
}

impl<'m> Iterator for PortIter<'m> {
    type Item = Port<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        let port = *self.iter.next()?;
        Some(port.to_port(self.module))
    }
}

pub struct ComponentRefIter<'m> {
    module: &'m Module,
    iter: hash_map::Iter<'m, StringId, ComponentRefId>,
}

impl<'m> Iterator for ComponentRefIter<'m> {
    type Item = (&'m str, ComponentRef<'m>);

    fn next(&mut self) -> Option<Self::Item> {
        let (&alias, &reference) = self.iter.next()?;
        let alias = &self.module.strings[alias];
        let reference = ComponentRef::new(self.module, reference);
        Some((alias, reference))
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

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Connection {
    source_component: Option<ComponentRefId>,
    source_pins: PortPins,
    sink_component: Option<ComponentRefId>,
    sink_pins: PortPins,
}

impl Connection {
    pub(crate) fn new(
        source_pins: PortPins,
        sink_pins: PortPins,
        source_component: Option<ComponentRefId>,
        sink_component: Option<ComponentRefId>,
    ) -> Self {
        Self {
            source_component,
            source_pins,
            sink_component,
            sink_pins,
        }
    }

    pub fn source_pins(&self) -> &PortPins {
        &self.source_pins
    }

    pub fn sink_pins(&self) -> &PortPins {
        &self.sink_pins
    }

    pub fn source_component<'m>(&self, module: &'m Module) -> Option<ComponentRef<'m>> {
        self.source_component
            .map(|source_component| source_component.to_reference(module))
    }

    pub fn sink_component<'m>(&self, module: &'m Module) -> Option<ComponentRef<'m>> {
        self.sink_component
            .map(|sink_component| sink_component.to_reference(module))
    }

    pub fn source_port<'m>(
        &self,
        module: &'m Module,
        component: &Component<'m>,
    ) -> Option<Port<'m>> {
        if let Some(source_component) = self.source_component {
            let source_component = source_component.to_component(module);
            source_component
                .ports()
                .find(|port| port.name() == self.source_pins.port(module).name())
        } else {
            component
                .ports()
                .find(|port| port.name() == self.source_pins.port(module).name())
        }
    }

    pub fn sink_port<'m>(&self, module: &'m Module, component: &Component<'m>) -> Option<Port<'m>> {
        if let Some(sink_component) = self.sink_component {
            let sink_component = sink_component.to_component(module);
            sink_component
                .ports()
                .find(|port| port.name() == self.sink_pins.port(module).name())
        } else {
            component
                .ports()
                .find(|port| port.name() == self.sink_pins.port(module).name())
        }
    }
}

pub struct ComponentBuilder<'m> {
    pub(crate) module: &'m mut Module,
    pub(crate) placeholder: ComponentId,
    pub(crate) data: ComponentData,
    unresolved_references: HashSet<ComponentWeakRef>,
    unresolved_named_references: HashMap<StringId, ComponentWeakRef>,
    unresolved_connections: Vec<WeakConnection>,
    name_is_set: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentWeakRef {
    pub(crate) component: StringId,
    pub(crate) alias: Option<StringId>,
    pub(crate) n_instances: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WeakConnection {
    pub(crate) source_pins: WeakPortPins,
    pub(crate) source_component: Option<StringId>,
    pub(crate) sink_pins: WeakPortPins,
    pub(crate) sink_component: Option<StringId>,
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

#[derive(Debug)]
pub struct ComponentBuildArtifacts {
    pub references: HashSet<ComponentWeakRef>,
    pub named_references: HashMap<StringId, ComponentWeakRef>,
    pub connections: HashSet<WeakConnection>,
}

pub type ComponentBuildResult = Result<(ComponentId, ComponentBuildArtifacts), ComponentBuildError>;

pub trait GetStringId {
    fn get_string_id(&self, module: &mut Module) -> StringId;
}

impl GetStringId for StringId {
    fn get_string_id(&self, _module: &mut Module) -> StringId {
        *self
    }
}

impl<S: AsRef<str>> GetStringId for S {
    fn get_string_id(&self, module: &mut Module) -> StringId {
        module.strings.entry(self.as_ref())
    }
}

impl<'m> ComponentBuilder<'m> {
    pub fn new(module: &'m mut Module) -> Self {
        let data = ComponentData::new(module, "", None);
        let placeholder = module.component_db.entry(data.clone());
        assert!(module.components.insert(data.name, placeholder).is_none());

        let unresolved_references = HashSet::new();
        let unresolved_named_references = HashMap::new();
        let unresolved_connections = Vec::new();

        Self {
            module,
            data,
            placeholder,
            unresolved_references,
            unresolved_named_references,
            unresolved_connections,
            name_is_set: false,
        }
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        let name = self.module.strings.entry(name);
        assert!(
            self.module.components.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = &self.module.strings[name],
            module = &self.module.strings[self.module.name]
        );

        if let Some(component) = self.module.components.remove(&self.data.name) {
            self.module.components.insert(name, component);
        } else {
            panic!("component not in module")
        }

        self.data.name = name;

        self.name_is_set = true;
        self
    }

    pub fn add_port(&mut self) -> PortBuilder<'_> {
        PortBuilder::new(self.module, self.placeholder)
    }

    pub fn add_reference(
        &mut self,
        component: ComponentId,
        alias: Option<&str>,
        n_instances: Option<usize>,
    ) -> Result<ComponentRefId, ComponentBuildError> {
        let alias = match alias {
            Some(alias) => self.module.strings.entry(alias),
            None => self.module[component].name,
        };
        let n_instances = n_instances.unwrap_or(1);

        let reference = ComponentRefData::new(component, alias, n_instances);
        let reference = self.module.reference_db.entry(reference);
        if self.data.references.insert(alias, reference).is_some() {
            let component = self.module.strings[self.module[component].name].to_string();
            let reference = self.module.strings[alias].to_string();
            Err(ComponentBuildError::DuplicateReference {
                component,
                reference,
            })
        } else {
            Ok(reference)
        }
    }

    pub fn add_weak_reference<S: GetStringId>(
        &mut self,
        component: S,
        alias: Option<S>,
        n_instances: Option<usize>,
    ) -> Result<ComponentWeakRef, ComponentBuildError> {
        let component = component.get_string_id(self.module);
        let alias = alias.map(|alias| alias.get_string_id(self.module));

        let reference = ComponentWeakRef {
            component,
            alias,
            n_instances: n_instances.unwrap_or(1),
        };

        if let Some(alias) = alias {
            if self
                .unresolved_named_references
                .insert(alias, reference)
                .is_some()
            {
                let parent = self.module.strings[self.data.name].to_string();
                let reference = self.module.strings[alias].to_string();
                return Err(ComponentBuildError::DuplicateReference {
                    component: parent,
                    reference,
                });
            }
        } else if !self.unresolved_references.insert(reference) {
            let parent = self.module.strings[self.data.name].to_string();
            let reference = self.module.strings[component].to_string();
            return Err(ComponentBuildError::DuplicateReference {
                component: parent,
                reference,
            });
        }

        Ok(reference)
    }

    pub fn add_connection(&mut self) -> ConnectionBuilder<'_> {
        ConnectionBuilder::new(&mut self.data)
    }

    pub fn add_weak_connection(&mut self) -> WeakConnectionBuilder<'_, 'm> {
        WeakConnectionBuilder::new(self)
    }

    pub fn set_class(&mut self, class: ComponentClass) -> &mut Self {
        self.data.class = Some(class);
        self
    }

    pub fn is_name_set(&self) -> bool {
        self.name_is_set
    }

    pub fn is_ports_empty(&self) -> bool {
        self.data.ports.is_empty()
    }

    pub fn is_references_empty(&self) -> bool {
        self.data.references.is_empty()
    }

    pub fn is_unresolved_references_empty(&self) -> bool {
        self.unresolved_references.is_empty()
    }

    pub fn is_unresolved_named_references_empty(&self) -> bool {
        self.unresolved_named_references.is_empty()
    }

    pub fn is_connections_empty(&self) -> bool {
        self.data.connections.is_empty()
    }

    pub fn is_class_set(&self) -> bool {
        self.data.class.is_some()
    }

    pub fn finish(mut self) -> ComponentBuildResult {
        use std::mem;

        if !self.is_name_set() {
            return Err(ComponentBuildError::MissingField("name"));
        }

        debug_assert!(self.data.ports.is_empty());
        mem::swap(
            &mut self.data.ports,
            &mut self.module[self.placeholder].ports,
        );

        // let name = self.data.name;
        // let component = self.module.component_db.entry(self.data);

        // if self.module.components.insert(name, component).is_some() {
        //     let component = self.module.strings[name].to_string();
        //     let module = self.module.strings[self.module.name].to_string();
        //     return Err(ComponentBuildError::DuplicateComponent { module, component });
        // }

        #[cfg(debug_assertions)]
        let name = self.data.name;

        self.module[self.placeholder] = self.data;

        #[cfg(debug_assertions)]
        {
            let component = self.module[self.placeholder].name(self.module);
            let component = self
                .module
                .strings
                .rlookup(component)
                .expect("component name should be in module strings");
            debug_assert_eq!(component, name);
            debug_assert!(self.module.components.contains_key(&component));
        }

        // TODO: check duplicate connections?
        let connections = HashSet::from_iter(self.unresolved_connections);

        Ok((
            self.placeholder,
            ComponentBuildArtifacts {
                references: self.unresolved_references,
                named_references: self.unresolved_named_references,
                connections,
            },
        ))
    }
}

pub struct ConnectionBuilder<'m> {
    component: &'m mut ComponentData,
    source: Option<(PortPins, Option<ComponentRefId>)>,
    sink: Option<(PortPins, Option<ComponentRefId>)>,
}

#[derive(Debug, Error)]
pub enum ConnectionBuildError {
    #[error("connection must have a {0}")]
    MissingField(&'static str),
    #[error(r#"undefined port "{port}" connected"#)]
    UndefinedPort { port: String },
    #[error(r#"undefined component "{reference}" connected"#)]
    UndefinedReference { reference: String },
}

impl<'m> ConnectionBuilder<'m> {
    fn new(component: &'m mut ComponentData) -> Self {
        Self {
            component,
            source: None,
            sink: None,
        }
    }

    pub fn set_source(&mut self, pins: PortPins, component: Option<ComponentRefId>) -> &mut Self {
        self.source = Some((pins, component));
        self
    }

    pub fn set_sink(&mut self, pins: PortPins, component: Option<ComponentRefId>) -> &mut Self {
        self.sink = Some((pins, component));
        self
    }

    pub fn is_source_set(&self) -> bool {
        self.source.is_some()
    }

    pub fn is_sink_set(&self) -> bool {
        self.sink.is_some()
    }

    pub fn finish(self) -> Result<&'m Connection, ConnectionBuildError> {
        let source = self
            .source
            .ok_or(ConnectionBuildError::MissingField("source"))?;
        let sink = self
            .sink
            .ok_or(ConnectionBuildError::MissingField("sink"))?;

        let connections = &mut self.component.connections;
        let i = connections.len();
        connections.push(Connection::new(source.0, sink.0, source.1, sink.1));

        Ok(&connections[i])
    }
}

pub struct WeakConnectionBuilder<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
    source: Option<(WeakPortPins, Option<StringId>)>,
    sink: Option<(WeakPortPins, Option<StringId>)>,
}

impl<'a, 'm> WeakConnectionBuilder<'a, 'm> {
    fn new(builder: &'a mut ComponentBuilder<'m>) -> Self {
        Self {
            builder,
            source: None,
            sink: None,
        }
    }

    pub fn set_source(
        &mut self,
        port: &str,
        range: Range<u32>,
        component: Option<&str>,
    ) -> &mut Self {
        let module = &mut self.builder.module;
        let port = module.strings.entry(port);
        let pins = WeakPortPins::new(port, range);
        let component = component.map(|component| module.strings.entry(component));
        self.source = Some((pins, component));
        self
    }

    pub fn set_sink(
        &mut self,
        port: &str,
        range: Range<u32>,
        component: Option<&str>,
    ) -> &mut Self {
        let module = &mut self.builder.module;
        let port = module.strings.entry(port);
        let pins = WeakPortPins::new(port, range);
        let component = component.map(|component| self.builder.module.strings.entry(component));
        self.sink = Some((pins, component));
        self
    }

    pub fn is_source_set(&self) -> bool {
        self.source.is_some()
    }

    pub fn is_sink_set(&self) -> bool {
        self.sink.is_some()
    }

    pub fn finish(self) -> Result<&'a WeakConnection, ConnectionBuildError> {
        let source = self
            .source
            .ok_or(ConnectionBuildError::MissingField("source"))?;
        let sink = self
            .sink
            .ok_or(ConnectionBuildError::MissingField("sink"))?;

        let connections = &mut self.builder.unresolved_connections;
        let i = connections.len();
        connections.push(WeakConnection {
            source_pins: source.0,
            source_component: source.1,
            sink_pins: sink.0,
            sink_component: sink.1,
        });

        Ok(&connections[i])
    }
}
