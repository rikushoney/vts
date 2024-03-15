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
    pub(crate) strings: StringTable<StringId>,
    pub(crate) components: Database<Component, ComponentId>,
    pub(crate) component_names: HashMap<StringId, ComponentId>,
    pub(crate) ports: Database<Port, PortId>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let mut strings = StringTable::default();
        let name = strings.entry(name);
        let components = Database::default();
        let component_names = HashMap::default();
        let ports = Database::default();

        Self {
            name,
            strings,
            components,
            component_names,
            ports,
        }
    }

    pub fn name(&self) -> &str {
        self.strings.lookup(self.name)
    }

    pub fn component(&self, component: ComponentId) -> &Component {
        assert!(self.component_names.values().any(|c| c == &component));
        self.components.lookup(component)
    }

    pub fn component_mut(&mut self, component: ComponentId) -> &mut Component {
        assert!(self.component_names.values().any(|c| c == &component));
        self.components.lookup_mut(component)
    }

    pub fn component_id(&self, name: &str) -> Option<ComponentId> {
        let name = self.strings.rlookup(name)?;
        self.component_names.get(&name).copied()
    }

    pub fn add_component(&mut self, recipe: &ComponentRecipe) -> ComponentId {
        let component = recipe.instantiate(self);

        debug_assert!(self.component_names.values().any(|c| c == &component));
        debug_assert!({
            let name = self
                .strings
                .rlookup(self.component(component).name(self))
                .expect("component should be instantiated");
            self.component_names.contains_key(&name)
        });

        component
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

impl Serialize for Module {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serializer = serializer.serialize_struct("Module", 2)?;

        let name = self.strings.lookup(self.name);
        serializer.serialize_field("name", name)?;

        let components_serializer = ComponentsSerializer {
            module: self,
            components: &self.components,
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
    fn test_module() {
        let mut module = Module::new("test_mod");

        let mut test_comp = ComponentRecipe::new();
        test_comp.name("test_comp");

        let mut test_port = PortRecipe::new();
        test_port.kind(PortKind::Input);
        test_comp.port(test_port.clone().name("test_port1"));
        test_comp.port(test_port.clone().name("test_port2"));

        let component = module.add_component(&test_comp);
        {
            let component = module.component(component);
            assert_eq!(component.name(&module), "test_comp");

            let port1 = {
                let id = component.port_id(&module, "test_port1").unwrap();
                component.port(&module, id)
            };
            assert_eq!(port1.name(&module), "test_port1");

            let port2 = {
                let id = component.port_id(&module, "test_port2").unwrap();
                component.port(&module, id)
            };
            assert_eq!(port2.name(&module), "test_port2");
        }
    }
}
