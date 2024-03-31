use std::fmt;

use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
};
use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use vts_core::arch::ComponentClass;

use crate::arch::{port::de::PyPortsDeserializer, PyComponent, PyComponentClass};

pub struct PyComponentDeserializer<'a, 'py> {
    py: Python<'py>,
    name: &'a String,
}

impl<'a, 'py> PyComponentDeserializer<'a, 'py> {
    pub fn new(py: Python<'py>, name: &'a String) -> Self {
        Self { py, name }
    }
}

impl<'a, 'de, 'py> DeserializeSeed<'de> for PyComponentDeserializer<'a, 'py> {
    type Value = Bound<'py, PyComponent>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentVisitor<'py> {
            name: Bound<'py, PyString>,
        }

        impl<'de, 'py> Visitor<'de> for ComponentVisitor<'py> {
            type Value = Bound<'py, PyComponent>;

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

                let mut ports: Option<Bound<'py, PyDict>> = None;
                let mut references: Option<()> = None;
                let mut class: Option<ComponentClass> = None;

                let py = self.name.py();

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Ports => {
                            if ports.is_some() {
                                return Err(de::Error::duplicate_field("ports"));
                            }
                            ports = Some(map.next_value_seed(PyPortsDeserializer::new(py))?);
                        }
                        Field::References => {
                            if references.is_some() {
                                return Err(de::Error::duplicate_field("references"));
                            }
                            // TODO: references
                            #[allow(clippy::let_unit_value)]
                            let _ = map.next_value()?;
                            references = Some(());
                        }
                        Field::Class => {
                            if class.is_some() {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            class = Some(map.next_value()?);
                        }
                    }
                }

                let class = class.map(PyComponentClass::from);
                let mut component = PyComponent::new(&self.name, class);

                if let Some(ports) = ports {
                    let ports = ports.as_mapping();
                    map_py_de_err!(component.add_ports(ports))?;
                }

                if let Some(_references) = references {
                    // TODO: add references
                }

                map_py_de_err!(Bound::new(py, component))
            }
        }

        let name = PyString::new_bound(self.py, self.name.as_str());
        deserializer.deserialize_struct(
            "Component",
            &["ports", "references", "class"],
            ComponentVisitor { name },
        )
    }
}

pub struct PyComponentsDeserializer<'py> {
    py: Python<'py>,
}

impl<'py> PyComponentsDeserializer<'py> {
    pub fn new(py: Python<'py>) -> Self {
        Self { py }
    }
}

impl<'de, 'py> DeserializeSeed<'de> for PyComponentsDeserializer<'py> {
    type Value = Bound<'py, PyDict>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentsVisitor<'py> {
            py: Python<'py>,
        }

        impl<'de, 'py> Visitor<'de> for ComponentsVisitor<'py> {
            type Value = Bound<'py, PyDict>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of component names to components")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let py = self.py;

                let components = PyDict::new_bound(py);
                while let Some(name) = map.next_key::<String>()? {
                    if map_py_de_err!(components.contains(name.as_str()))? {
                        return Err(de::Error::custom(format!(
                            r#"duplicate component "{name}""#
                        )));
                    }

                    let component = map.next_value_seed(PyComponentDeserializer::new(py, &name))?;
                    map_py_de_err!(components.set_item(name, component))?;
                }

                Ok(components)
            }
        }

        deserializer.deserialize_map(ComponentsVisitor { py: self.py })
    }
}
