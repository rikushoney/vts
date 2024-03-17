use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::{
    impl_dbkey_wrapper,
    port::{PortId, PortsDeserializer, PortsSerializer},
    Module, Port, StringId,
};
use crate::database::Database;

impl_dbkey_wrapper!(ComponentId, u32);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub(crate) name: StringId,
    pub(crate) ports: HashMap<StringId, PortId>,
    references: HashMap<StringId, ComponentId>,
    pub class: Option<ComponentClass>,
}

impl Component {
    fn new(module: &mut Module, name: &str, class: Option<ComponentClass>) -> Component {
        let name = module.strings.entry(name);
        assert!(
            module.components.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = module.strings.lookup(name),
            module = module.strings.lookup(module.name)
        );

        let ports = HashMap::default();
        let references = HashMap::default();

        Self {
            name,
            ports,
            references,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.lookup(self.name)
    }

    pub fn set_name<'m>(&'m mut self, module: &'m mut Module, name: &str) {
        let name = module.strings.entry(name);
        assert!(
            module.components.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = module.strings.lookup(name),
            module = module.strings.lookup(module.name)
        );

        let component = module
            .components
            .remove(&self.name)
            .expect("component should be in module");
        module.components.insert(name, component);
        self.name = name;
    }

    pub fn port<'m>(&self, module: &'m Module, port: PortId) -> &'m Port {
        assert!(
            self.ports.values().any(|p| p == &port),
            r#"port "{port}" not in component "{component}""#,
            port = module.port_db.lookup(port).name(module),
            component = self.name(module),
        );
        module.port_db.lookup(port)
    }

    pub fn port_mut<'m>(&'m self, module: &'m mut Module, port: PortId) -> &'m mut Port {
        assert!(
            self.ports.values().any(|p| p == &port),
            r#"port "{port}" not in component "{component}""#,
            port = module.port_db.lookup(port).name(module),
            component = self.name(module),
        );
        module.port_db.lookup_mut(port)
    }

    pub fn port_id(&self, module: &Module, name: &str) -> Option<PortId> {
        let name = module.strings.rlookup(name)?;
        self.ports.get(&name).copied()
    }
}

struct ComponentRefsSerializer<'a, 'm> {
    module: &'m Module,
    references: &'a HashMap<StringId, ComponentId>,
}

impl<'a, 'm> Serialize for ComponentRefsSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(self.references.len()))?;

        #[allow(clippy::for_kv_map)]
        for (name, _component) in self.references {
            let name = self.module.strings.lookup(*name);
            serializer.serialize_element(name)?;
        }

        serializer.end()
    }
}

pub struct ComponentSerializer<'m> {
    module: &'m Module,
    component: &'m Component,
}

impl<'m> ComponentSerializer<'m> {
    pub fn new(module: &'m Module, component: &'m Component) -> Self {
        Self { module, component }
    }
}

impl<'m> Serialize for ComponentSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Component", 4)?;

        let ports_serializer = PortsSerializer::new(self.module, &self.component.ports);
        serializer.serialize_field("ports", &ports_serializer)?;

        let component_refs_serializer = ComponentRefsSerializer {
            module: self.module,
            references: &self.component.references,
        };
        serializer.serialize_field("references", &component_refs_serializer)?;

        serializer.serialize_field("class", &self.component.class)?;

        serializer.end()
    }
}

pub struct ComponentsSerializer<'m> {
    module: &'m Module,
    components: &'m Database<Component, ComponentId>,
}

impl<'m> ComponentsSerializer<'m> {
    pub fn new(module: &'m Module, components: &'m Database<Component, ComponentId>) -> Self {
        Self { module, components }
    }
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

pub struct ComponentDeserializer<'m, 'de> {
    module: &'m mut Module,
    name: &'de str,
}

impl<'m, 'de> ComponentDeserializer<'m, 'de> {
    pub fn new(module: &'m mut Module, name: &'de str) -> Self {
        Self { module, name }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for ComponentDeserializer<'m, 'de> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentVisitor<'m, 'de> {
            module: &'m mut Module,
            name: &'de str,
        }

        const FIELDS: &[&str] = &["name", "ports", "references", "class"];

        impl<'de, 'm> Visitor<'de> for ComponentVisitor<'m, 'de> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a component description")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "lowercase")]
                enum Field {
                    Ports,
                    References,
                    Class,
                }

                let mut ports = false;
                let mut references = false;
                let mut class = false;

                let mut component = Component::new(self.module, self.name, None);

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Ports => {
                            if ports {
                                return Err(de::Error::duplicate_field("ports"));
                            }
                            map.next_value_seed(PortsDeserializer::new(
                                self.module,
                                &mut component,
                            ))?;
                            ports = true;
                        }
                        Field::References => {
                            if references {
                                return Err(de::Error::duplicate_field("references"));
                            }
                            // TODO: deserialize references
                            references = true;
                        }
                        Field::Class => {
                            if class {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            component.class = Some(map.next_value()?);
                            class = true;
                        }
                    }
                }

                let name = component.name;
                let component = self.module.component_db.entry(component);

                assert!(
                    self.module.components.insert(name, component).is_none(),
                    r#"component "{component}" already in module "{module}""#,
                    component = self.module.strings.lookup(name),
                    module = self.module.strings.lookup(name),
                );

                debug_assert!(self.module.components.values().any(|c| c == &component));
                debug_assert!({
                    let name = self
                        .module
                        .strings
                        .rlookup(self.module.component(component).name(self.module))
                        .expect("component name should be in module strings");
                    self.module.components.contains_key(&name)
                });

                Ok(())
            }
        }

        deserializer.deserialize_struct(
            "Component",
            FIELDS,
            ComponentVisitor {
                module: self.module,
                name: self.name,
            },
        )
    }
}

pub struct ComponentsDeserializer<'m> {
    module: &'m mut Module,
}

impl<'m> ComponentsDeserializer<'m> {
    pub fn new(module: &'m mut Module) -> Self {
        Self { module }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for ComponentsDeserializer<'m> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentsVisitor<'m> {
            module: &'m mut Module,
        }

        impl<'de, 'm> Visitor<'de> for ComponentsVisitor<'m> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of component descriptions")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while let Some(name) = map.next_key()? {
                    map.next_value_seed(ComponentDeserializer::new(self.module, name))?;
                }

                Ok(())
            }
        }

        deserializer.deserialize_map(ComponentsVisitor {
            module: self.module,
        })
    }
}
