use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{
    component::{ComponentBuildError, ComponentBuilder},
    port::de::PortsDeserializer,
    ComponentId, Module, StringId,
};

pub(crate) struct ComponentRefsDeserializer<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
}

impl<'a, 'm> ComponentRefsDeserializer<'a, 'm> {
    pub(crate) fn new(builder: &'a mut ComponentBuilder<'m>) -> Self {
        Self { builder }
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for ComponentRefsDeserializer<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentRefsVisitor<'m> {
            module: &'m mut Module,
        }

        impl<'de, 'm> Visitor<'de> for ComponentRefsVisitor<'m> {
            type Value = Vec<StringId>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a list of component references")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut references = match seq.size_hint() {
                    Some(size) => Vec::with_capacity(size),
                    None => Vec::new(),
                };

                while let Some(reference) = seq.next_element()? {
                    let name = self.module.strings.entry(reference);
                    if references.contains(&name) {
                        return Err(de::Error::custom(format!(
                            r#"duplicate component reference "{reference}""#
                        )));
                    }

                    references.push(name);
                }

                Ok(references)
            }
        }

        let unresolved = deserializer.deserialize_seq(ComponentRefsVisitor {
            module: self.builder.module,
        })?;

        for reference in unresolved {
            self.builder.weak_reference(reference);
        }

        Ok(())
    }
}

pub struct ComponentDeserializer<'m> {
    module: &'m mut Module,
    name: String,
}

impl<'m> ComponentDeserializer<'m> {
    pub fn new(module: &'m mut Module, name: String) -> Self {
        Self { module, name }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for ComponentDeserializer<'m> {
    type Value = (ComponentId, Vec<StringId>);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentVisitor<'a, 'm> {
            module: &'m mut Module,
            name: &'a str,
        }

        const FIELDS: &[&str] = &["ports", "references", "class"];

        impl<'a, 'de, 'm> Visitor<'de> for ComponentVisitor<'a, 'm> {
            type Value = (ComponentId, Vec<StringId>);

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

                let mut builder = ComponentBuilder::new(self.module);
                builder.name(self.name);

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Ports => {
                            if builder.has_ports() {
                                return Err(de::Error::duplicate_field("ports"));
                            }
                            map.next_value_seed(PortsDeserializer::new(&mut builder))?;
                        }
                        Field::References => {
                            if builder.has_unresolved_references() {
                                return Err(de::Error::duplicate_field("references"));
                            }
                            map.next_value_seed(ComponentRefsDeserializer::new(&mut builder))?;
                        }
                        Field::Class => {
                            if builder.has_class() {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            builder.class(map.next_value()?);
                        }
                    }
                }

                let (component, references) = match builder.finish() {
                    Ok((component, references)) => (component, references),
                    Err(err) => match err {
                        ComponentBuildError::DuplicateComponent { module, component } => {
                            return Err(de::Error::custom(format!(
                                r#"component "{component}" already in module "{module}""#,
                            )))
                        }
                        ComponentBuildError::DuplicateReference { .. } => {
                            unreachable!("duplicate references should be handled by ComponentRefsDeserializer")
                        }
                        ComponentBuildError::MissingField(name) => {
                            return Err(de::Error::missing_field(name));
                        }
                    },
                };

                Ok((component, references))
            }
        }

        deserializer.deserialize_struct(
            "Component",
            FIELDS,
            ComponentVisitor {
                module: self.module,
                name: &self.name,
            },
        )
    }
}

pub struct ComponentsDeserializer<'m> {
    module: &'m mut Module,
}

impl<'m> ComponentsDeserializer<'m> {
    pub fn new(module: &'m mut Module) -> Self {
        Self { module }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for ComponentsDeserializer<'m> {
    type Value = HashMap<ComponentId, Vec<StringId>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentsVisitor<'m> {
            module: &'m mut Module,
        }

        impl<'de, 'm> Visitor<'de> for ComponentsVisitor<'m> {
            type Value = HashMap<ComponentId, Vec<StringId>>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of component descriptions")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut component_refs = match map.size_hint() {
                    Some(count) => HashMap::with_capacity(count),
                    None => HashMap::default(),
                };

                while let Some(name) = map.next_key()? {
                    let (component, references) =
                        map.next_value_seed(ComponentDeserializer::new(self.module, name))?;
                    component_refs.insert(component, references);
                }

                Ok(component_refs)
            }
        }

        deserializer.deserialize_map(ComponentsVisitor {
            module: self.module,
        })
    }
}
