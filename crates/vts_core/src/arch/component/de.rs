use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{component::ComponentData, port::de::PortsDeserializer, Module};

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

        const FIELDS: &[&str] = &["ports", "references", "class"];

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

                let mut component = ComponentData::new(self.module, self.name, None);

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
                            #[allow(clippy::let_unit_value)]
                            let _ = map.next_value()?;
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
