use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{port::PortData, Module, Port, StringId};

pub struct PortSerializer<'m> {
    // TODO: is this needed?
    _module: &'m Module,
    port: &'m PortData,
}

impl<'m> PortSerializer<'m> {
    pub fn new(module: &'m Module, port: &'m PortData) -> Self {
        Self {
            _module: module,
            port,
        }
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
        port_serializer.serialize_field("class", &self.port.class)?;

        port_serializer.end()
    }
}

pub struct PortsSerializer<'m> {
    module: &'m Module,
    ports: &'m HashMap<StringId, Port>,
}

impl<'m> PortsSerializer<'m> {
    pub fn new(module: &'m Module, ports: &'m HashMap<StringId, Port>) -> Self {
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
            let name = self.module.strings.lookup(*name);
            let port = self.module.port_db.lookup(*port);
            serializer.serialize_entry(
                name,
                &PortSerializer {
                    _module: self.module,
                    port,
                },
            )?;
        }

        serializer.end()
    }
}
