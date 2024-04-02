use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{port::PortData, Module, PortId, StringId};

pub struct PortSerializer<'m> {
    port: &'m PortData,
}

impl<'m> PortSerializer<'m> {
    pub fn new(port: &'m PortData) -> Self {
        Self { port }
    }
}

impl<'m> Serialize for PortSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut port_serializer = serializer.serialize_struct("Port", 4)?;

        port_serializer.serialize_field("kind", &self.port.kind)?;
        port_serializer.serialize_field("n_pins", &self.port.n_pins)?;

        if let Some(class) = self.port.class {
            port_serializer.serialize_field("class", &class)?;
        }

        port_serializer.end()
    }
}

pub struct PortsSerializer<'m> {
    module: &'m Module,
    ports: &'m HashMap<StringId, PortId>,
}

impl<'m> PortsSerializer<'m> {
    pub fn new(module: &'m Module, ports: &'m HashMap<StringId, PortId>) -> Self {
        Self { module, ports }
    }
}

impl<'m> Serialize for PortsSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.ports.len()))?;

        for (name, port) in self.ports {
            let name = &self.module.strings[*name];
            let port = &self.module.port_db[*port];
            serializer.serialize_entry(name, &PortSerializer { port })?;
        }

        serializer.end()
    }
}
