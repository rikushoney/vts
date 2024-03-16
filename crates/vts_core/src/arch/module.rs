use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, MapAccess, Visitor},
    ser::{SerializeMap, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
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

struct ModuleVisitor;

impl<'de> Visitor<'de> for ModuleVisitor {
    type Value = Module;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a module definition")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        enum Field {
            Name,
            Components,
        }

        let mut name: Option<&str> = None;
        let mut components: Option<HashMap<&str, ComponentRecipe>> = None;

        while let Some(key) = map.next_key()? {
            match key {
                Field::Name => {
                    if name.is_some() {
                        return Err(de::Error::duplicate_field("name"));
                    }
                    name = Some(map.next_value()?);
                }
                Field::Components => {
                    if components.is_some() {
                        return Err(de::Error::duplicate_field("components"));
                    }
                    components = Some(map.next_value()?);
                }
            }
        }

        let name = match name {
            Some(name) => name,
            None => {
                return Err(de::Error::missing_field("name"));
            }
        };
        let components = components.unwrap_or_default();

        let mut module = Module::new(name);

        for (name, mut recipe) in components {
            debug_assert!(recipe.name.is_none());
            recipe.name = Some(name.to_string());
            module.add_component(&recipe);
        }

        Ok(module)
    }
}

impl<'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Module", &["name", "components"], ModuleVisitor)
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
