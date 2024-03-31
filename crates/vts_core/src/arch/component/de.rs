use std::collections::{HashMap, HashSet};
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
                    let reference = self.module.strings.entry(reference);
                    if references.contains(&reference) {
                        let alias = self.module.strings.lookup(reference);
                        return Err(de::Error::custom(format!(
                            r#"duplicate component reference "{alias}""#
                        )));
                    }

                    references.push(reference);
                }

                Ok(references)
            }
        }

        deserializer.deserialize_seq(ComponentRefsVisitor {
            module: self.builder.module,
        })
    }
}

pub(crate) struct ComponentNamedRefsDeserializer<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
}

impl<'a, 'm> ComponentNamedRefsDeserializer<'a, 'm> {
    pub(crate) fn new(builder: &'a mut ComponentBuilder<'m>) -> Self {
        Self { builder }
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for ComponentNamedRefsDeserializer<'a, 'm> {
    type Value = HashMap<StringId, StringId>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentNamedRefsVisitor<'m> {
            module: &'m mut Module,
        }

        impl<'de, 'm> Visitor<'de> for ComponentNamedRefsVisitor<'m> {
            type Value = HashMap<StringId, StringId>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a list of named component references")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut references = match seq.size_hint() {
                    Some(size) => HashMap::with_capacity(size),
                    None => HashMap::default(),
                };

                while let Some((alias, component)) = seq.next_element()? {
                    let reference = self.module.strings.entry(alias);
                    let component = self.module.strings.entry(component);

                    if references.insert(reference, component).is_some() {
                        return Err(de::Error::custom(format!(
                            r#"duplicate component reference "{alias}""#
                        )));
                    }
                }

                Ok(references)
            }
        }

        deserializer.deserialize_seq(ComponentNamedRefsVisitor {
            module: self.builder.module,
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
    type Value = (ComponentId, HashSet<StringId>, HashMap<StringId, StringId>);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentVisitor<'a, 'm> {
            module: &'m mut Module,
            name: &'a str,
        }

        const FIELDS: &[&str] = &["ports", "references", "named_references", "class"];

        impl<'a, 'de, 'm> Visitor<'de> for ComponentVisitor<'a, 'm> {
            type Value = (ComponentId, HashSet<StringId>, HashMap<StringId, StringId>);

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
                    #[serde(rename = "named_references")]
                    NamedReferences,
                    Class,
                }

                let mut builder = ComponentBuilder::new(self.module);
                builder.set_name(self.name);

                let mut ok = Ok(());
                'outer: while let Some(key) = map.next_key()? {
                    match key {
                        Field::Ports => {
                            if !builder.is_ports_empty() {
                                return Err(de::Error::duplicate_field("ports"));
                            }
                            map.next_value_seed(PortsDeserializer::new(&mut builder))?;
                        }
                        Field::References => {
                            if !builder.is_unresolved_references_empty() {
                                return Err(de::Error::duplicate_field("references"));
                            }
                            for reference in
                                map.next_value_seed(ComponentRefsDeserializer::new(&mut builder))?
                            {
                                ok = builder.add_weak_reference(reference, None).map(|_| ());
                                if ok.is_err() {
                                    break 'outer;
                                }
                            }
                        }
                        Field::NamedReferences => {
                            if !builder.is_unresolved_named_references_empty() {
                                return Err(de::Error::duplicate_field("named references"));
                            }
                            for (alias, component) in map.next_value_seed(
                                ComponentNamedRefsDeserializer::new(&mut builder),
                            )? {
                                ok = builder
                                    .add_weak_reference(component, Some(alias))
                                    .map(|_| ());
                                if ok.is_err() {
                                    break 'outer;
                                }
                            }
                        }
                        Field::Class => {
                            if builder.is_class_set() {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            builder.set_class(map.next_value()?);
                        }
                    }
                }

                let (component, references, named_references) = match ok.and(builder.finish()) {
                    Ok((component, references, named_references)) => {
                        (component, references, named_references)
                    }
                    Err(err) => match err {
                        ComponentBuildError::DuplicateComponent { module, component } => {
                            return Err(de::Error::custom(format!(
                                r#"component "{component}" already in module "{module}""#,
                            )))
                        }
                        ComponentBuildError::DuplicateReference { .. } => {
                            unreachable!("duplicate references should be handled by ComponentRefsDeserializer/ComponentNamedRefsDeserializer")
                        }
                        ComponentBuildError::MissingField(name) => {
                            return Err(de::Error::missing_field(name));
                        }
                    },
                };

                Ok((component, references, named_references))
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
    type Value = HashMap<ComponentId, (HashSet<StringId>, HashMap<StringId, StringId>)>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentsVisitor<'m> {
            module: &'m mut Module,
        }

        impl<'de, 'm> Visitor<'de> for ComponentsVisitor<'m> {
            type Value = HashMap<ComponentId, (HashSet<StringId>, HashMap<StringId, StringId>)>;

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
                    let (component, references, named_references) =
                        map.next_value_seed(ComponentDeserializer::new(self.module, name))?;
                    component_refs.insert(component, (references, named_references));
                }

                Ok(component_refs)
            }
        }

        deserializer.deserialize_map(ComponentsVisitor {
            module: self.module,
        })
    }
}
