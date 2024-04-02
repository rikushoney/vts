use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{
    component::{ComponentData, ComponentRefData, ComponentRefId},
    port::ser::PortsSerializer,
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
