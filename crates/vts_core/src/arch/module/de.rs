use std::fmt;

use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{component::de::ComponentsDeserializer, Module};

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
