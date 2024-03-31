use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{
    component::de::ComponentsDeserializer,
    module::{ModuleBuildError, ModuleBuilder},
    Module,
};

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
                            if builder.has_name() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            builder.name(map.next_value()?);
                        }
                        Field::Components => {
                            if builder.has_components() {
                                return Err(de::Error::duplicate_field("components"));
                            }
                            unresolved = map.next_value_seed(ComponentsDeserializer::new(
                                &mut builder.module,
                            ))?;
                        }
                    }
                }

                let mut ok = Ok(());
                for (component, references) in unresolved {
                    let references = references.into_iter();
                    if let Err(err) = builder.resolve_references(component, references) {
                        ok = Err(err);
                        break;
                    }
                }

                ok.and(builder.finish()).map_err(|err| match err {
                    ModuleBuildError::MissingField(name) => de::Error::missing_field(name),
                    ModuleBuildError::DuplicateReference {
                        component,
                        reference,
                    } => de::Error::custom(format!(
                        r#"component "{reference}" already referenced in "{component}""#,
                    )),
                    ModuleBuildError::UndefinedReference {
                        component,
                        reference,
                    } => de::Error::custom(format!(
                        r#"undefined component "{reference}" referenced in "{component}""#,
                    )),
                })
            }
        }

        deserializer.deserialize_struct("Module", &["name", "components"], ModuleVisitor)
    }
}
