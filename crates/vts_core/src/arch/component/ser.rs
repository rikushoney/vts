use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{
    component::{ComponentData, ComponentRefData, ComponentRefId, Connection},
    port::{ser::PortsSerializer, PortPins},
    ComponentId, Module, StringId,
};
use crate::database::Database;

struct ComponentRefSerializer<'m> {
    module: &'m Module,
    reference: &'m ComponentRefData,
}

impl<'m> Serialize for ComponentRefSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut component_ref_serializer = serializer.serialize_struct("ComponentRef", 1)?;
        let component = self.reference.component.to_component(self.module);

        component_ref_serializer.serialize_field("component", component.name())?;
        component_ref_serializer.serialize_field("n_instances", &self.reference.n_instances)?;

        component_ref_serializer.end()
    }
}

struct ComponentRefsSerializer<'a, 'm> {
    module: &'m Module,
    references: &'a HashMap<StringId, ComponentRefId>,
}

impl<'a, 'm> Serialize for ComponentRefsSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(self.references.len()))?;

        for (&alias, &reference) in self.references.iter() {
            let reference = &self.module[reference];
            let name = self.module[reference.component].name;
            if name == alias {
                let component_ref_serializer = ComponentRefSerializer {
                    module: self.module,
                    reference,
                };

                serializer.serialize_element(&component_ref_serializer)?;
            }
        }

        serializer.end()
    }
}

struct ComponentNamedRefsSerializer<'a, 'm> {
    module: &'m Module,
    references: &'a HashMap<StringId, ComponentRefId>,
}

impl<'a, 'm> Serialize for ComponentNamedRefsSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.references.len()))?;

        for (&alias, &reference) in self.references.iter() {
            let reference = &self.module[reference];
            let name = self.module[reference.component].name;
            if name != alias {
                let component_ref_serializer = ComponentRefSerializer {
                    module: self.module,
                    reference,
                };
                let alias = &self.module.strings[alias];

                serializer.serialize_entry(alias, &component_ref_serializer)?;
            }
        }

        serializer.end()
    }
}

struct InterfaceSerializer<'a, 'm> {
    module: &'m Module,
    pins: &'a PortPins,
    component: Option<ComponentRefId>,
}

impl<'a, 'm> Serialize for InterfaceSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = self.component.map(|_| 5).unwrap_or(4);
        let mut serializer = serializer.serialize_struct("Interface", len)?;

        let port = self.pins.port(self.module);
        serializer.serialize_field("port", port.name())?;

        let start = self.pins.start();
        serializer.serialize_field("port_start", &start)?;
        let end = self.pins.end();
        serializer.serialize_field("port_end", &end)?;

        if let Some(component) = self.component {
            let reference = component.to_reference(self.module);
            serializer.serialize_field("component", reference.alias())?;
        }

        serializer.end()
    }
}

pub struct ConnectionSerializer<'m> {
    module: &'m Module,
    connection: &'m Connection,
}

impl<'m> ConnectionSerializer<'m> {
    pub fn new(module: &'m Module, connection: &'m Connection) -> Self {
        Self { module, connection }
    }
}

impl<'m> Serialize for ConnectionSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Connection", 2)?;

        let source_serializer = InterfaceSerializer {
            module: self.module,
            pins: &self.connection.source_pins,
            component: self.connection.source_component,
        };
        serializer.serialize_field("source", &source_serializer)?;

        let sink_serializer = InterfaceSerializer {
            module: self.module,
            pins: &self.connection.sink_pins,
            component: self.connection.sink_component,
        };
        serializer.serialize_field("sink", &sink_serializer)?;

        serializer.end()
    }
}

struct ConnectionsSerializer<'m> {
    module: &'m Module,
    connections: &'m Vec<Connection>,
}

impl<'m> Serialize for ConnectionsSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(self.connections.len()))?;

        for connection in self.connections {
            let connection_serializer = ConnectionSerializer::new(self.module, connection);
            serializer.serialize_element(&connection_serializer)?;
        }

        serializer.end()
    }
}

pub struct ComponentSerializer<'m> {
    module: &'m Module,
    component: &'m ComponentData,
}

impl<'m> ComponentSerializer<'m> {
    pub fn new(module: &'m Module, component: &'m ComponentData) -> Self {
        Self { module, component }
    }
}

impl<'m> Serialize for ComponentSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Component", 5)?;

        if !self.component.ports.is_empty() {
            let ports_serializer = PortsSerializer::new(self.module, &self.component.ports);
            serializer.serialize_field("ports", &ports_serializer)?;
        }

        if !self.component.references.is_empty() {
            let component_refs_serializer = ComponentRefsSerializer {
                module: self.module,
                references: &self.component.references,
            };
            serializer.serialize_field("references", &component_refs_serializer)?;

            let component_named_refs_serializer = ComponentNamedRefsSerializer {
                module: self.module,
                references: &self.component.references,
            };
            serializer.serialize_field("named_references", &component_named_refs_serializer)?;
        }

        if !self.component.connections.is_empty() {
            let connections_serializer = ConnectionsSerializer {
                module: self.module,
                connections: &self.component.connections,
            };
            serializer.serialize_field("connections", &connections_serializer)?;
        }

        if let Some(class) = self.component.class {
            serializer.serialize_field("class", &class)?;
        }

        serializer.end()
    }
}

pub struct ComponentsSerializer<'m> {
    module: &'m Module,
    components: &'m Database<ComponentData, ComponentId>,
}

impl<'m> ComponentsSerializer<'m> {
    pub fn new(module: &'m Module, components: &'m Database<ComponentData, ComponentId>) -> Self {
        Self { module, components }
    }
}

impl<'m> Serialize for ComponentsSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.components.len()))?;

        for component in self.components.values() {
            let name = &self.module.strings[component.name];
            serializer.serialize_entry(
                name,
                &ComponentSerializer {
                    module: self.module,
                    component,
                },
            )?;
        }

        serializer.end()
    }
}
