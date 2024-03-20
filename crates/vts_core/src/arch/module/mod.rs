pub mod de;
pub mod ser;

use std::collections::HashMap;

use crate::arch::{component::ComponentData, port::Port, port::PortData, Component, StringId};
use crate::{database::Database, stringtable::StringTable, OpaqueKey};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub(crate) name: StringId,
    pub(crate) strings: StringTable<StringId>,
    pub(crate) component_db: Database<ComponentData, Component>,
    pub(crate) components: HashMap<StringId, Component>,
    pub(crate) port_db: Database<PortData, Port>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let mut strings = StringTable::default();
        let name = strings.entry(name);
        let component_db = Database::default();
        let components = HashMap::default();
        let port_db = Database::default();

        Self {
            name,
            strings,
            component_db,
            components,
            port_db,
        }
    }

    pub fn name(&self) -> &str {
        self.strings.lookup(self.name)
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = self.strings.entry(name);
    }

    pub fn component(&self, component: Component) -> &ComponentData {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.component_db.lookup(component)
    }

    pub fn component_mut(&mut self, component: Component) -> &mut ComponentData {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.component_db.lookup_mut(component)
    }

    pub fn component_id(&self, name: &str) -> Option<Component> {
        let name = self.strings.rlookup(name)?;
        self.components.get(&name).copied()
    }

    pub fn get_data<T: DataId>(&self, id: T) -> &T::Data {
        T::get_data(self, id)
    }
}

pub trait DataId {
    type Data;

    fn get_data(module: &Module, id: Self) -> &Self::Data;
}

impl DataId for Port {
    type Data = PortData;

    fn get_data(module: &Module, id: Self) -> &Self::Data {
        module.port_db.lookup(id)
    }
}

impl DataId for Component {
    type Data = ComponentData;

    fn get_data(module: &Module, id: Self) -> &Self::Data {
        module.component_db.lookup(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module() {
        let mut _module = Module::new("test_mod");
    }
}
