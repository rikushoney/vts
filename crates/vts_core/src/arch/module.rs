use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::{
    component::{ComponentId, ComponentsDeserializer, ComponentsSerializer},
    port::PortId,
    Component, Port, StringId,
};
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

impl Serialize for Module {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut serializer = serializer.serialize_struct("Module", 2)?;

        let name = self.strings.lookup(self.name);
        serializer.serialize_field("name", name)?;

        let components_serializer = ComponentsSerializer::new(self, &self.component_db);
        serializer.serialize_field("components", &components_serializer)?;

        serializer.end()
    }
}

impl<'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
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
                #[serde(rename_all = "lowercase")]
                enum Field {
                    Name,
                    Components,
                }

                let mut module = Module::new("");

                let mut name = false;
                let mut components = false;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            module.set_name(map.next_value()?);
                            name = true;
                        }
                        Field::Components => {
                            if components {
                                return Err(de::Error::duplicate_field("components"));
                            }
                            map.next_value_seed(ComponentsDeserializer::new(&mut module))?;
                            components = true;
                        }
                    }
                }

                if !name {
                    return Err(de::Error::missing_field("name"));
                }

                Ok(module)
            }
        }

        deserializer.deserialize_struct("Module", &["name", "components"], ModuleVisitor)
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
