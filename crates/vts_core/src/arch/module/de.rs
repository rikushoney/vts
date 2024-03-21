use std::collections::HashMap;
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
                let mut components = None;

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
                            if components.is_some() {
                                return Err(de::Error::duplicate_field("components"));
                            }
                            components = Some(
                                map.next_value_seed(ComponentsDeserializer::new(&mut module))?,
                            );
                        }
                    }
                }

                if !name {
                    return Err(de::Error::missing_field("name"));
                }

                if let Some(components) = components {
                    for (component, references) in components {
                        let mut resolved = HashMap::with_capacity(references.len());
                        for name in references {
                            if let Some(reference) = module.components.get(&name) {
                                assert!(
                                    resolved.insert(name, reference.reference()).is_none(),
                                    r#"component "{reference}" already referenced in "{component}""#,
                                    reference = module.strings.lookup(name),
                                    component = module.component(component).name(&module)
                                );
                            } else {
                                return Err(de::Error::custom(
                                    format!(
                                        r#"undefined component "{reference}" referenced in "{component}""#,
                                        reference = module.strings.lookup(name),
                                        component = module.component(component).name(&module)
                                    )
                                    .as_str(),
                                ));
                            }
                        }

                        module
                            .component_mut(component)
                            .references
                            .extend(resolved.into_iter());
                    }
                }

                Ok(module)
            }
        }

        deserializer.deserialize_struct("Module", &["name", "components"], ModuleVisitor)
    }
}
