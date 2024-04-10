#![allow(unused)] // TODO: remove this!

use std::fmt;
use std::slice;

use serde::ser::SerializeMap;
use serde::ser::SerializeSeq;
use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use thiserror::Error;

use super::{
    connection::Connection,
    linker::Linker,
    port::{PortSeed, WeakPortPins},
    reference::{ComponentWeakRef, DeserializeComponentWeakRef},
    ComponentId, ComponentRef, ComponentRefId, Module, Port, PortId,
};

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
pub struct Component<'m>(&'m Module, ComponentId);

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

    // pub fn ports(&self) -> PortIter<'m> {
    //     PortIter {
    //         module: self.module,
    //         iter: self.data.ports.values(),
    //     }
    // }

    // pub fn references(&self) -> ComponentRefIter<'m> {
    //     ComponentRefIter {
    //         module: self.module,
    //         iter: self.data.references.iter(),
    //     }
    // }

    // pub fn connections(&self) -> ConnectionIter<'m> {
    //     ConnectionIter {
    //         iter: self.data.connections.iter(),
    //     }
    // }

    pub fn class(&self) -> Option<ComponentClass> {
        self.data().class
    }

    pub fn find_port(&self, name: &str) -> Option<Port<'_>> {
        self.data()
            .ports
            .iter()
            .find(|&port| self.module()[*port].name == name)
            .map(|&port| Port::new(self.module(), port))
    }
}

// pub struct PortIter<'m> {
//     module: &'m Module,
//     iter: hash_map::Values<'m, String, PortId>,
// }

// impl<'m> Iterator for PortIter<'m> {
//     type Item = Port<'m>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let port = *self.iter.next()?;
//         Some(port.to_port(self.module))
//     }
// }

// pub struct ComponentRefIter<'m> {
//     module: &'m Module,
//     iter: hash_map::Iter<'m, String, ComponentRefId>,
// }

// impl<'m> Iterator for ComponentRefIter<'m> {
//     type Item = (&'m str, ComponentRef<'m>);

//     fn next(&mut self) -> Option<Self::Item> {
//         let (&alias, &reference) = self.iter.next()?;
//         let alias = &self.module.strings[alias];
//         let reference = ComponentRef::new(self.module, reference);
//         Some((alias, reference))
//     }
// }

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
    pub fn finish(self) -> Component<'m> {
        let component = {
            let component = ComponentData::new(&self.name.0, self.class);

            // TODO: check duplicate components
            self.module.components.insert(component)
        };

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

const FIELDS: &[&str] = &[
    "ports",
    "references",
    "named_references",
    "connections",
    "class",
];

const PORTS: usize = 0;
const REFERENCES: usize = 1;
const NAMED_REFERENCES: usize = 2;
const CONNECTIONS: usize = 3;
const CLASS: usize = 4;

struct SerializePorts<'a, 'm> {
    module: &'m Module,
    ports: &'a Vec<PortId>,
}

impl<'m> Serialize for SerializePorts<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.ports.len()))?;

        for &port in self.ports {
            let port = Port::new(self.module, port);
            state.serialize_entry(port.name(), port.data())?;
        }

        state.end()
    }
}

struct SerializeReferences<'a, 'm> {
    module: &'m Module,
    references: &'a Vec<ComponentRefId>,
}

impl<'m> Serialize for SerializeReferences<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let unnamed_references = self
            .references
            .iter()
            .filter(|&reference| self.module[*reference].alias.is_none());

        let len = unnamed_references.clone().count();
        let mut state = serializer.serialize_seq(Some(len))?;

        if len == 0 {
            return state.end();
        }

        for &reference in unnamed_references {
            let reference = ComponentRef::new(self.module, reference);

            if reference.alias().is_none() {
                state.serialize_element(&ComponentWeakRef {
                    component: reference.component().name().to_string(),
                    alias: None,
                    n_instances: reference.n_instances(),
                })?;
            }
        }

        state.end()
    }
}

struct SerializeNamedReferences<'a, 'm> {
    module: &'m Module,
    references: &'a Vec<ComponentRefId>,
}

impl<'m> Serialize for SerializeNamedReferences<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let named_references = self
            .references
            .iter()
            .filter(|&reference| self.module[*reference].alias.is_some());

        let len = named_references.clone().count();
        let mut state = serializer.serialize_map(Some(len))?;

        for &reference in named_references {
            let reference = ComponentRef::new(self.module, reference);
            let alias = reference.alias().expect("reference should have an alias");

            state.serialize_entry(
                alias,
                &ComponentWeakRef {
                    component: reference.component().name().to_string(),
                    alias: None,
                    n_instances: reference.n_instances(),
                },
            )?;
        }

        state.end()
    }
}

struct SerializeConnections<'a, 'm> {
    module: &'m Module,
    connections: &'a Vec<Connection>,
}

impl<'m> Serialize for SerializeConnections<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        todo!()
    }
}

pub(crate) struct SerializeComponent<'m> {
    component: Component<'m>,
}

impl<'m> SerializeComponent<'m> {
    pub(crate) fn new(module: &'m Module, component: ComponentId) -> Self {
        Self {
            component: Component::new(module, component),
        }
    }
}

impl<'m> Serialize for SerializeComponent<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Component", FIELDS.len())?;

        state.serialize_field(
            FIELDS[PORTS],
            &SerializePorts {
                module: self.component.module(),
                ports: &self.component.data().ports,
            },
        )?;

        state.serialize_field(
            FIELDS[REFERENCES],
            &SerializeReferences {
                module: self.component.module(),
                references: &self.component.data().references,
            },
        )?;

        state.serialize_field(
            FIELDS[NAMED_REFERENCES],
            &SerializeNamedReferences {
                module: self.component.module(),
                references: &self.component.data().references,
            },
        )?;

        state.serialize_field(
            FIELDS[CONNECTIONS],
            &SerializeConnections {
                module: self.component.module(),
                connections: &self.component.data().connections,
            },
        )?;

        state.serialize_field(FIELDS[CLASS], &self.component.class())?;
        state.end()
    }
}

struct DeserializePorts<'m> {
    module: &'m mut Module,
    parent: ComponentId,
}

impl<'de, 'm> Visitor<'de> for DeserializePorts<'m> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a dict of {}", FIELDS[PORTS])
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(port) = map.next_key::<String>()? {
            map.next_value_seed(PortSeed::new(self.module, self.parent, port))?;
        }

        Ok(())
    }
}

impl<'de, 'm> DeserializeSeed<'de> for DeserializePorts<'m> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

struct DeserializeReferences<'a, 'm> {
    module: &'m mut Module,
    parent: ComponentId,
    linker: &'a mut Linker,
}

impl<'a, 'de, 'm> Visitor<'de> for DeserializeReferences<'a, 'm> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a dict of {}", FIELDS[REFERENCES])
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(reference) = seq.next_element_seed(DeserializeComponentWeakRef::Unnamed)? {
            self.linker
                .add_reference(ComponentKey::new(self.parent), reference);
        }

        Ok(())
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for DeserializeReferences<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct DeserializeNamedReferences<'a, 'm> {
    module: &'m mut Module,
    parent: ComponentId,
    linker: &'a mut Linker,
}

impl<'a, 'de, 'm> Visitor<'de> for DeserializeNamedReferences<'a, 'm> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a dict of {}", FIELDS[NAMED_REFERENCES])
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(alias) = map.next_key()? {
            let reference = map.next_value_seed(DeserializeComponentWeakRef::Named(alias))?;
            self.linker
                .add_reference(ComponentKey::new(self.parent), reference);
        }

        Ok(())
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for DeserializeNamedReferences<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

struct DeserializeConnections<'a, 'm> {
    module: &'m mut Module,
    parent: ComponentId,
    linker: &'a mut Linker,
}

impl<'a, 'de, 'm> Visitor<'de> for DeserializeConnections<'a, 'm> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a list of {}", FIELDS[CONNECTIONS])
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        todo!()
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for DeserializeConnections<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

pub(crate) struct ComponentSeed<'a, 'm> {
    module: &'m mut Module,
    name: String,
    linker: &'a mut Linker,
}

impl<'a, 'm> ComponentSeed<'a, 'm> {
    pub(crate) fn new(module: &'m mut Module, name: String, linker: &'a mut Linker) -> Self {
        Self {
            module,
            name,
            linker,
        }
    }
}

impl<'a, 'de, 'm> Visitor<'de> for ComponentSeed<'a, 'm> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a component description")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum Field {
            Ports,
            References,
            #[serde(rename = "named_references")]
            NamedReferences,
            Connections,
            Class,
        }

        let component = ComponentBuilder::new(self.module)
            .set_name(&self.name)
            .finish()
            .1;

        let mut ports = false;
        let mut references = false;
        let mut named_references = false;
        let mut connections = false;
        let mut class: Option<ComponentClass> = None;

        while let Some(field) = map.next_key()? {
            match field {
                Field::Ports => {
                    if ports {
                        return Err(de::Error::duplicate_field("ports"));
                    }

                    map.next_value_seed(DeserializePorts {
                        module: self.module,
                        parent: component,
                    })?;

                    ports = true;
                }
                Field::References => {
                    if references {
                        return Err(de::Error::duplicate_field("references"));
                    }

                    map.next_value_seed(DeserializeReferences {
                        module: self.module,
                        parent: component,
                        linker: self.linker,
                    })?;

                    references = true;
                }
                Field::NamedReferences => {
                    if named_references {
                        return Err(de::Error::duplicate_field("named_references"));
                    }

                    map.next_value_seed(DeserializeNamedReferences {
                        module: self.module,
                        parent: component,
                        linker: self.linker,
                    })?;

                    named_references = true;
                }
                Field::Connections => {
                    if connections {
                        return Err(de::Error::duplicate_field("connections"));
                    }

                    map.next_value_seed(DeserializeConnections {
                        module: self.module,
                        parent: component,
                        linker: self.linker,
                    })?;

                    connections = true;
                }
                Field::Class => {
                    if class.is_some() {
                        return Err(de::Error::duplicate_field("class"));
                    }

                    class = Some(map.next_value()?);
                }
            }
        }

        self.module[component].class = class;
        Ok(())
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for ComponentSeed<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Component", FIELDS, self)
    }
}
