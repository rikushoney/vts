use std::cell::RefCell;
use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeStruct},
    Serialize, Serializer,
};

use super::{
    component::{ComponentId, ComponentRecipe, ComponentSerializer},
    port::PortId,
    Component, Port, StringId,
};
use crate::{database::Database, stringtable::StringTable};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub(crate) name: StringId,
    pub(crate) strings: RefCell<StringTable<StringId>>,
    pub(crate) components: RefCell<Database<Component, ComponentId>>,
    pub(crate) component_name_map: RefCell<HashMap<StringId, ComponentId>>,
    pub(crate) ports: RefCell<Database<Port, PortId>>,
    pub(crate) port_name_map: RefCell<HashMap<StringId, PortId>>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let mut strings = RefCell::new(StringTable::default());
        let name = strings.borrow_mut().entry(name);
        let components = RefCell::default();
        let component_name_map = RefCell::default();
        let ports = RefCell::default();
        let port_name_map = RefCell::default();

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
        self.strings.borrow().lookup(self.name)
    }

    pub fn component(&self, name: &str) -> Option<&Component> {
        let name = self.strings.borrow().rlookup(name)?;
        let id = *self.component_name_map.borrow().get(&name)?;

        Some({
            let components = self.components.borrow();
            components.lookup(id)
        })
    }

    pub fn add_component(&mut self, recipe: &ComponentRecipe) -> &Component {
        recipe.instantiate(self)
    }
}

struct ComponentsSerializer<'m> {
    module: &'m Module,
    components: &'m Database<Component, ComponentId>,
}

impl<'m> Serialize for ComponentsSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.components.len()))?;

        for (_id, component) in self.components.iter() {
            let name = self.module.strings.borrow().lookup(component.name);
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

impl Serialize for Module {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serializer = serializer.serialize_struct("Module", 2)?;

        let name = self.strings.borrow().lookup(self.name);
        serializer.serialize_field("name", name)?;

        let components_serializer = ComponentsSerializer {
            module: self,
            components: &self.components.borrow(),
        };
        serializer.serialize_field("components", &components_serializer)?;

        serializer.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::{port::PortRecipe, PortKind};

    #[test]
    fn it_works() {
        let mut module = Module::new("test_mod");
        let mut test_comp = ComponentRecipe::new();
        test_comp.name("test_comp");
        let mut test_port = PortRecipe::new();
        test_port.kind(PortKind::Input);
        test_comp.port(test_port.clone().name("test_port1"));
        test_comp.port(test_port.clone().name("test_port2"));
        let component = module.add_component(&test_comp);
        assert_eq!(component.name(&module), "test_comp");
    }
}
