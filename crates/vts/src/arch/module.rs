use std::collections::HashMap;

use crate::{database::Database, stringtable::StringTable};

use crate::arch::{impl_dbkey_wrapper, Component, Port, StringId};

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
