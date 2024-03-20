use std::fmt;

use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
};
use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use vts_core::arch::{PortClass, PortKind};

use crate::arch::{map_py_de_err, PyPort, PyPortClass, PyPortKind};

pub struct PyPortDeserializer<'a, 'py> {
    py: Python<'py>,
    name: &'a String,
}

impl<'a, 'py> PyPortDeserializer<'a, 'py> {
    pub fn new(py: Python<'py>, name: &'a String) -> Self {
        Self { py, name }
    }
}

impl<'a, 'de, 'py> DeserializeSeed<'de> for PyPortDeserializer<'a, 'py> {
    type Value = Py<PyPort>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PortVisitor<'py> {
            py: Python<'py>,
            name: Py<PyString>,
        }

        impl<'de, 'py> Visitor<'de> for PortVisitor<'py> {
            type Value = Py<PyPort>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a port description")
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
                    Some(kind) => PyPortKind::from(kind),
                    None => {
                        return Err(de::Error::missing_field("kind"));
                    }
                };
                let n_pins = Some(n_pins.unwrap_or(1));
                let class = class.map(PyPortClass::from);

                let port = map_py_de_err!(PyPort::new(self.py, self.name, kind, n_pins, class))?;
                map_py_de_err!(Py::new(self.py, port))
            }
        }

        let name = PyString::new(self.py, self.name.as_str());
        deserializer.deserialize_struct(
            "Port",
            &["kind", "n_pins", "class"],
            PortVisitor {
                py: self.py,
                name: name.into_py(self.py),
            },
        )
    }
}

pub struct PyPortsDeserializer<'py> {
    py: Python<'py>,
}

impl<'py> PyPortsDeserializer<'py> {
    pub fn new(py: Python<'py>) -> Self {
        Self { py }
    }
}

impl<'de, 'py> DeserializeSeed<'de> for PyPortsDeserializer<'py> {
    type Value = Py<PyDict>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PortsVisitor<'py> {
            py: Python<'py>,
        }

        impl<'de, 'py> Visitor<'de> for PortsVisitor<'py> {
            type Value = Py<PyDict>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of port names to ports")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let ports = PyDict::new(self.py);
                while let Some(name) = map.next_key::<String>()? {
                    if map_py_de_err!(ports.contains(name.as_str()))? {
                        return Err(de::Error::custom(format!(r#"duplicate port "{name}""#)));
                    }

                    let port = map.next_value_seed(PyPortDeserializer::new(self.py, &name))?;
                    map_py_de_err!(ports.set_item(name, port))?;
                }

                let ports: Py<PyDict> = ports.into_py(self.py);
                Ok(ports)
            }
        }

        deserializer.deserialize_map(PortsVisitor { py: self.py })
    }
}
