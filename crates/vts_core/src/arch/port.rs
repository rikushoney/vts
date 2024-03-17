use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    ser::{SerializeMap, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::{component::ComponentId, impl_dbkey_wrapper, Component, Module, StringId};

impl_dbkey_wrapper!(PortId, u32);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum PortClass {
    #[serde(rename = "CLOCK")]
    Clock,
    #[serde(rename = "LUT_IN")]
    LutIn,
    #[serde(rename = "LUT_OUT")]
    LutOut,
    #[serde(rename = "LATCH_IN")]
    LatchIn,
    #[serde(rename = "LATCH_OUT")]
    LatchOut,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    name: StringId,
    parent: ComponentId,
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}

impl Port {
    fn new(
        module: &mut Module,
        parent: &mut Component,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> Port {
        let name = module.strings.entry(name);
        assert!(parent.ports.get(&name).is_none(), "{}", {
            let name = module.strings.lookup(name);
            let component_name = module.strings.lookup(parent.name);
            format!(r#"port "{name}" already in component "{component_name}""#)
        });

        let parent = *module
            .components
            .get(&parent.name)
            .expect("parent component not in module");

        Self {
            name,
            parent,
            kind,
            n_pins,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.lookup(self.name)
    }

    pub fn set_name<'m>(&'m mut self, module: &'m mut Module, name: &str) {
        let name = module.strings.entry(name);
        let parent = module.component_mut(self.parent);
        assert!(parent.ports.get(&name).is_none(), "{}", {
            let name = module.strings.lookup(name);
            let module_name = module.strings.lookup(module.name);
            format!(r#"component "{name}" already in module "{module_name}""#)
        });

        let port = parent
            .ports
            .remove(&self.name)
            .expect("port should be in module");
        parent.ports.insert(name, port);
        self.name = name;
    }
}

pub struct PortSerializer<'m> {
    // TODO: is this needed?
    _module: &'m Module,
    port: &'m Port,
}

impl<'m> PortSerializer<'m> {
    pub fn new(module: &'m Module, port: &'m Port) -> Self {
        Self {
            _module: module,
            port,
        }
    }
}

impl<'m> Serialize for PortSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut port_serializer = serializer.serialize_struct("Port", 4)?;

        port_serializer.serialize_field("kind", &self.port.kind)?;
        port_serializer.serialize_field("n_pins", &self.port.n_pins)?;
        port_serializer.serialize_field("class", &self.port.class)?;

        port_serializer.end()
    }
}

pub struct PortsSerializer<'m> {
    module: &'m Module,
    ports: &'m HashMap<StringId, PortId>,
}

impl<'m> PortsSerializer<'m> {
    pub fn new(module: &'m Module, ports: &'m HashMap<StringId, PortId>) -> Self {
        Self { module, ports }
    }
}

impl<'m> Serialize for PortsSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.ports.len()))?;

        for (name, port) in self.ports {
            let name = self.module.strings.lookup(*name);
            let port = self.module.port_db.lookup(*port);
            serializer.serialize_entry(
                name,
                &PortSerializer {
                    _module: self.module,
                    port,
                },
            )?;
        }

        serializer.end()
    }
}

pub struct PortDeserializer<'de, 'm> {
    module: &'m mut Module,
    name: &'de str,
    component: &'m mut Component,
}

impl<'de, 'm> PortDeserializer<'de, 'm> {
    pub(crate) fn new(
        module: &'m mut Module,
        name: &'de str,
        component: &'m mut Component,
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
            component: &'m mut Component,
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

                let port = Port::new(self.module, self.component, self.name, kind, n_pins, class);

                let name = port.name;
                let port = self.module.port_db.entry(port);

                let prev = self.component.ports.insert(name, port);
                assert!(prev.is_none(), "{}", {
                    let port_name = self.module.strings.lookup(name);
                    let module_name = self.module.strings.lookup(name);
                    format!(r#"port "{port_name}" already in module "{module_name}""#)
                });

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
    component: &'m mut Component,
}

impl<'m> PortsDeserializer<'m> {
    pub fn new(module: &'m mut Module, component: &'m mut Component) -> Self {
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
            component: &'m mut Component,
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
