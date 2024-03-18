use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{component::ComponentData, port::PortData, Module, PortClass, PortKind};

pub struct PortDeserializer<'de, 'm> {
    module: &'m mut Module,
    name: &'de str,
    component: &'m mut ComponentData,
}

impl<'de, 'm> PortDeserializer<'de, 'm> {
    pub(crate) fn new(
        module: &'m mut Module,
        name: &'de str,
        component: &'m mut ComponentData,
    ) -> Self {
        Self {
            module,
            name,
            component,
        }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for PortDeserializer<'de, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PortVisitor<'de, 'm> {
            module: &'m mut Module,
            name: &'de str,
            component: &'m mut ComponentData,
        }

        const FIELDS: &[&str] = &["name", "kind", "n_pins", "class"];

        impl<'de, 'm> Visitor<'de> for PortVisitor<'de, 'm> {
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
                let mut kind: Option<PortKind> = None;
                let mut n_pins: Option<usize> = None;
                let mut class: Option<PortClass> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Kind => {
                            if kind.is_some() {
                                return Err(de::Error::duplicate_field("kind"));
                            }
                            kind = Some(map.next_value()?);
                        }
                        Field::Npins => {
                            if n_pins.is_some() {
                                return Err(de::Error::duplicate_field("n_pins"));
                            }
                            n_pins = Some(map.next_value()?);
                        }
                        Field::Class => {
                            if class.is_some() {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            class = Some(map.next_value()?);
                        }
                    }
                }

                let kind = match kind {
                    Some(kind) => kind,
                    None => {
                        return Err(de::Error::missing_field("kind"));
                    }
                };
                let n_pins = n_pins.unwrap_or(1);

                let port =
                    PortData::new(self.module, self.component, self.name, kind, n_pins, class);

                let name = port.name;
                let port = self.module.port_db.entry(port);

                assert!(
                    self.component.ports.insert(name, port).is_none(),
                    r#"port "{port}" already in module "{module}""#,
                    port = self.module.strings.lookup(name),
                    module = self.module.strings.lookup(name),
                );

                debug_assert!(self.component.ports.values().any(|p| p == &port));
                debug_assert!({
                    let name = self
                        .module
                        .strings
                        .rlookup(self.component.port(self.module, port).name(self.module))
                        .expect("port name should be in module strings");
                    self.component.ports.contains_key(&name)
                });

                Ok(())
            }
        }

        deserializer.deserialize_struct(
            "Port",
            FIELDS,
            PortVisitor {
                name: self.name,
                component: self.component,
                module: self.module,
            },
        )
    }
}

pub struct PortsDeserializer<'m> {
    module: &'m mut Module,
    component: &'m mut ComponentData,
}

impl<'m> PortsDeserializer<'m> {
    pub fn new(module: &'m mut Module, component: &'m mut ComponentData) -> Self {
        Self { module, component }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for PortsDeserializer<'m> {
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
            module: self.module,
            component: self.component,
        })
    }
}
