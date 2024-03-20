use std::fmt;

use pyo3::{prelude::*, types::PyDict};
use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{component::de::PyComponentsDeserializer, map_py_de_err, PyModule as PyModule_};

pub struct ModuleDeserializer<'py> {
    py: Python<'py>,
}

impl<'de, 'py> ModuleDeserializer<'py> {
    pub fn new(py: Python<'py>) -> Self {
        Self { py }
    }
}

impl<'de, 'py> DeserializeSeed<'de> for ModuleDeserializer<'py> {
    type Value = Py<PyModule_>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModuleVisitor<'py> {
            py: Python<'py>,
        }

        impl<'de, 'py> Visitor<'de> for ModuleVisitor<'py> {
            type Value = Py<PyModule_>;

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

                let mut name: Option<&str> = None;
                let mut components: Option<Py<PyDict>> = None;

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
                            components =
                                Some(map.next_value_seed(PyComponentsDeserializer::new(self.py))?);
                        }
                    }
                }

                let name = match name {
                    Some(name) => name,
                    None => {
                        return Err(de::Error::missing_field("name"));
                    }
                };

                let mut module = PyModule_::new(self.py, name);

                if let Some(components) = components {
                    let components = components.as_ref(self.py).as_mapping();
                    map_py_de_err!(module.add_components(self.py, components))?;
                }

                map_py_de_err!(Py::new(self.py, module))
            }
        }

        deserializer.deserialize_struct(
            "Module",
            &["name", "components"],
            ModuleVisitor { py: self.py },
        )
    }
}
