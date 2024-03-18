use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{
    component::ComponentId, port::ser::PortsSerializer, Component, Module, StringId,
};
use crate::database::Database;

struct ComponentRefsSerializer<'a, 'm> {
    module: &'m Module,
    references: &'a HashMap<StringId, ComponentId>,
}

impl<'a, 'm> Serialize for ComponentRefsSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(self.references.len()))?;

        #[allow(clippy::for_kv_map)]
        for (name, _component) in self.references {
            let name = self.module.strings.lookup(*name);
            serializer.serialize_element(name)?;
        }

        serializer.end()
    }
}

pub struct ComponentSerializer<'m> {
    module: &'m Module,
    component: &'m Component,
}

impl<'m> ComponentSerializer<'m> {
    pub fn new(module: &'m Module, component: &'m Component) -> Self {
        Self { module, component }
    }
}

impl<'m> Serialize for ComponentSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Component", 4)?;

        let ports_serializer = PortsSerializer::new(self.module, &self.component.ports);
        serializer.serialize_field("ports", &ports_serializer)?;

        let component_refs_serializer = ComponentRefsSerializer {
            module: self.module,
            references: &self.component.references,
        };
        serializer.serialize_field("references", &component_refs_serializer)?;

        serializer.serialize_field("class", &self.component.class)?;

        serializer.end()
    }
}

pub struct ComponentsSerializer<'m> {
    module: &'m Module,
    components: &'m Database<Component, ComponentId>,
}

impl<'m> ComponentsSerializer<'m> {
    pub fn new(module: &'m Module, components: &'m Database<Component, ComponentId>) -> Self {
        Self { module, components }
    }
}

impl<'m> Serialize for ComponentsSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.components.len()))?;

        for (_id, component) in self.components.iter() {
            let name = self.module.strings.lookup(component.name);
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
