use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{impl_dbkey_wrapper, Component, Port, StringId};
use crate::{database::Database, stringtable::StringTable};

impl_dbkey_wrapper!(ComponentId, u32);
impl_dbkey_wrapper!(PortId, u32);

#[derive(Clone, Debug, PartialEq)]
pub struct Module<'m> {
    pub(crate) name: StringId,
    pub(crate) strings: StringTable<StringId>,
    pub(crate) components: Database<Component<'m>, ComponentId>,
    pub(crate) component_name_map: HashMap<StringId, ComponentId>,
    pub(crate) ports: Database<Port<'m>, PortId>,
    pub(crate) port_name_map: HashMap<StringId, PortId>,
}

impl<'m> Module<'m> {
    pub fn new(name: &str) -> Self {
        let mut strings = StringTable::default();
        let name = strings.entry(name);
        let components = Database::default();
        let component_name_map = HashMap::default();
        let ports = Database::default();
        let port_name_map = HashMap::default();

        Self {
            name,
            strings,
            components,
            component_name_map,
            ports,
            port_name_map,
        }
    }

    pub fn name(&self) -> &str {
        self.strings.lookup(self.name)
    }

    pub fn component(&self, name: &str) -> Option<&Component<'m>> {
        let name = self.strings.rlookup(name)?;
        let id = *self.component_name_map.get(&name)?;

        Some(self.components.lookup(id))
    }
}

struct ComponentsSerializer<'a, 'm> {
    components: &'a Database<Component<'m>, ComponentId>,
}

impl<'a, 'm> Serialize for ComponentsSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.components.len()))?;

        for (_id, component) in self.components.iter() {
            let name = component.module.strings.lookup(component.name);
            serializer.serialize_entry(name, component)?;
        }

        serializer.end()
    }
}

impl<'m> Serialize for Module<'m> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serializer = serializer.serialize_struct("Module", 2)?;

        let name = self.strings.lookup(self.name);
        serializer.serialize_field("name", name)?;

        let components_serializer = ComponentsSerializer {
            components: &self.components,
        };
        serializer.serialize_field("components", &components_serializer)?;

        serializer.end()
    }
}
