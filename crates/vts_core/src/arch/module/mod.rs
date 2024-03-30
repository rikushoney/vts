pub mod de;
pub mod ser;

use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use crate::arch::{component::ComponentData, port::PortData, port::PortId, ComponentId, StringId};
use crate::{database::Database, stringtable::StringTable, OpaqueKey};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub(crate) name: StringId,
    pub(crate) strings: StringTable<StringId>,
    pub(crate) component_db: Database<ComponentData, ComponentId>,
    pub(crate) components: HashMap<StringId, ComponentId>,
    pub(crate) port_db: Database<PortData, PortId>,
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

    pub fn component(&self, component: ComponentId) -> &ComponentData {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.get_data(component)
    }

    pub fn component_mut(&mut self, component: ComponentId) -> &mut ComponentData {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.get_data_mut(component)
    }

    pub fn get_data<T: DataId>(&self, id: T) -> &T::Data {
        T::get_data(self, id)
    }

    pub fn get_data_mut<T: DataId>(&mut self, id: T) -> &mut T::Data {
        T::get_data_mut(self, id)
    }
}

pub trait DataId {
    type Data;

    fn get_data(module: &Module, id: Self) -> &Self::Data;

    fn get_data_mut(module: &mut Module, id: Self) -> &mut Self::Data;
}

impl DataId for PortId {
    type Data = PortData;

    fn get_data(module: &Module, id: Self) -> &Self::Data {
        module.port_db.lookup(id)
    }

    fn get_data_mut(module: &mut Module, id: Self) -> &mut Self::Data {
        module.port_db.lookup_mut(id)
    }
}

impl DataId for ComponentId {
    type Data = ComponentData;

    fn get_data(module: &Module, id: Self) -> &Self::Data {
        module.component_db.lookup(id)
    }

    fn get_data_mut(module: &mut Module, id: Self) -> &mut Self::Data {
        module.component_db.lookup_mut(id)
    }
}

impl<I: DataId> Index<I> for Module {
    type Output = I::Data;

    fn index(&self, id: I) -> &Self::Output {
        I::get_data(self, id)
    }
}

impl<I: DataId> IndexMut<I> for Module {
    fn index_mut(&mut self, id: I) -> &mut Self::Output {
        I::get_data_mut(self, id)
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
