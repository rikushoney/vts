pub mod de;
pub mod ser;

use std::collections::HashMap;

use crate::arch::{component::ComponentId, port::PortId, Component, Port, StringId};
use crate::{database::Database, stringtable::StringTable, OpaqueKey};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub(crate) name: StringId,
    pub(crate) strings: StringTable<StringId>,
    pub(crate) component_db: Database<Component, ComponentId>,
    pub(crate) components: HashMap<StringId, ComponentId>,
    pub(crate) port_db: Database<Port, PortId>,
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

    pub fn component(&self, component: ComponentId) -> &Component {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.component_db.lookup(component)
    }

    pub fn component_mut(&mut self, component: ComponentId) -> &mut Component {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.component_db.lookup_mut(component)
    }

    pub fn component_id(&self, name: &str) -> Option<ComponentId> {
        let name = self.strings.rlookup(name)?;
        self.components.get(&name).copied()
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
