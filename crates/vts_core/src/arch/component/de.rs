use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{
    component::ComponentData, port::de::PortsDeserializer, ComponentId, Module, StringId,
};

pub(crate) struct ComponentRefsDeserializer<'m> {
    module: &'m mut Module,
}

impl<'m> ComponentRefsDeserializer<'m> {
    pub(crate) fn new(module: &'m mut Module) -> Self {
        Self { module }
    }
}

impl<'de, 'm> DeserializeSeed<'de> for ComponentRefsDeserializer<'m> {
    type Value = Vec<StringId>;

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
                        return Err(de::Error::custom(
                            format!(r#"duplicate component reference "{reference}""#).as_str(),
                        ));
                    }

                    references.push(name);
                }

                Ok(references)
            }
        }

        deserializer.deserialize_seq(ComponentRefsVisitor {
            module: self.module,
        })
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

                let mut ports = false;
                let mut references: Option<Vec<StringId>> = None;
                let mut class = false;

                let mut component = ComponentData::new(self.module, self.name, None);

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Ports => {
                            if ports {
                                return Err(de::Error::duplicate_field("ports"));
                            }
                            map.next_value_seed(PortsDeserializer::new(
                                self.module,
                                &mut component,
                            ))?;
                            ports = true;
                        }
                        Field::References => {
                            if references.is_some() {
                                return Err(de::Error::duplicate_field("references"));
                            }
                            references = Some(
                                map.next_value_seed(ComponentRefsDeserializer::new(self.module))?,
                            );
                        }
                        Field::Class => {
                            if class {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            component.class = Some(map.next_value()?);
                            class = true;
                        }
                    }
                }

                let name = component.name;
                let component = self.module.component_db.entry(component);

                assert!(
                    self.module.components.insert(name, component).is_none(),
                    r#"component "{component}" already in module "{module}""#,
                    component = self.module.strings.lookup(name),
                    module = self.module.strings.lookup(name),
                );

                debug_assert!(self.module.components.values().any(|c| c == &component));
                debug_assert!({
                    let name = self
                        .module
                        .strings
                        .rlookup(self.module.component(component).name(self.module))
                        .expect("component name should be in module strings");
                    self.module.components.contains_key(&name)
                });

                Ok((component, references.unwrap_or_default()))
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
