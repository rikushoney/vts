use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{component::de::ComponentsDeserializer, module::ModuleBuilder, Module};

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

                let mut builder = ModuleBuilder::new();
                let mut unresolved = HashMap::default();

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if builder.is_name_set() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            builder.set_name(map.next_value()?);
                        }
                        Field::Components => {
                            if !builder.is_components_empty() {
                                return Err(de::Error::duplicate_field("components"));
                            }
                            unresolved = map.next_value_seed(ComponentsDeserializer::new(
                                &mut builder.module,
                            ))?;
                        }
                    }
                }

                builder
                    .resolve_and_finish(unresolved)
                    .map_err(|err| de::Error::custom(format!("{err}")))
            }
        }

        deserializer.deserialize_struct("Module", &["name", "components"], ModuleVisitor)
    }
}
