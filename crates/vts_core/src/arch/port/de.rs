use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{
    component::{ComponentBuilder, ComponentData},
    port::{PortBuildError, PortBuilder},
    Module,
};

pub struct PortDeserializer<'m> {
    module: &'m mut Module,
    name: String,
    component: &'m mut ComponentData,
}

impl<'m> PortDeserializer<'m> {
    pub(crate) fn new(
        module: &'m mut Module,
        name: String,
        component: &'m mut ComponentData,
    ) -> Self {
        Self {
            module,
            name,
            component,
        }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for PortDeserializer<'m> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PortVisitor<'a, 'm> {
            module: &'m mut Module,
            name: &'a str,
            component: &'m mut ComponentData,
        }

        const FIELDS: &[&str] = &["name", "kind", "n_pins", "class"];

        impl<'a, 'de, 'm> Visitor<'de> for PortVisitor<'a, 'm> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a port definition")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "lowercase")]
                enum Field {
                    Kind,
                    #[serde(rename = "n_pins")]
                    Npins,
                    Class,
                }

                let mut builder = PortBuilder::new(self.module, self.component);
                builder.name(self.name);

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Kind => {
                            if builder.has_kind() {
                                return Err(de::Error::duplicate_field("kind"));
                            }
                            builder.kind(map.next_value()?);
                        }
                        Field::Npins => {
                            if builder.has_n_pins() {
                                return Err(de::Error::duplicate_field("n_pins"));
                            }
                            builder.n_pins(map.next_value()?);
                        }
                        Field::Class => {
                            if builder.has_class() {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            builder.class(map.next_value()?);
                        }
                    }
                }

                if let Err(err) = builder.finish() {
                    Err(match err {
                        PortBuildError::MissingField(name) => de::Error::missing_field(name),
                        PortBuildError::DuplicatePort { port, module } => de::Error::custom(
                            format!(r#"port "{port}" already in module "{module}""#),
                        ),
                    })
                } else {
                    Ok(())
                }
            }
        }

        deserializer.deserialize_struct(
            "Port",
            FIELDS,
            PortVisitor {
                name: &self.name,
                component: self.component,
                module: self.module,
            },
        )
    }
}

pub struct PortsDeserializer<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
}

impl<'a, 'm> PortsDeserializer<'a, 'm> {
    pub fn new(builder: &'a mut ComponentBuilder<'m>) -> Self {
        Self { builder }
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for PortsDeserializer<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PortsVisitor<'m> {
            module: &'m mut Module,
            component: &'m mut ComponentData,
        }

        impl<'de, 'm> Visitor<'de> for PortsVisitor<'m> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of port descriptions")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while let Some(name) = map.next_key()? {
                    map.next_value_seed(PortDeserializer::new(self.module, name, self.component))?;
                }

                Ok(())
            }
        }

        deserializer.deserialize_map(PortsVisitor {
            module: self.builder.module,
            component: &mut self.builder.data,
        })
    }
}
