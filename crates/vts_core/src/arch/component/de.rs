use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{
    component::{ComponentBuilder, ComponentWeakRef},
    port::de::PortsDeserializer,
    ComponentId, Module, StringId,
};

pub(crate) struct ComponentRefDeserializer<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
    alias: Option<&'a str>,
}

impl<'a, 'm> ComponentRefDeserializer<'a, 'm> {
    pub(crate) fn new(builder: &'a mut ComponentBuilder<'m>, alias: Option<&'a str>) -> Self {
        Self { builder, alias }
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for ComponentRefDeserializer<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentRefVisitor<'a, 'm> {
            builder: &'a mut ComponentBuilder<'m>,
            alias: Option<&'a str>,
        }

        const FIELDS: &[&str] = &["n_instances"];

        impl<'a, 'de, 'm> Visitor<'de> for ComponentRefVisitor<'a, 'm> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a component reference")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "lowercase")]
                enum Fields {
                    Component,
                    #[serde(rename = "n_instances")]
                    NInstances,
                }

                let mut component: Option<String> = None;
                let mut n_instances = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::Component => {
                            if component.is_some() {
                                return Err(serde::de::Error::duplicate_field(stringify!(
                                    component
                                )));
                            }
                            component = Some(map.next_value()?);
                        }
                        Fields::NInstances => {
                            if n_instances.is_some() {
                                return Err(serde::de::Error::duplicate_field(stringify!(
                                    n_instances
                                )));
                            }
                            n_instances = Some(map.next_value()?);
                        }
                    }
                }

                let component = component.ok_or(de::Error::missing_field("component"))?;
                self.builder
                    .add_weak_reference(component.as_str(), self.alias, n_instances)
                    .map_err(|err| de::Error::custom(format!("{err}")))?;

                Ok(())
            }
        }

        deserializer.deserialize_struct(
            "ComponentRef",
            FIELDS,
            ComponentRefVisitor {
                builder: self.builder,
                alias: self.alias,
            },
        )
    }
}

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
        struct ComponentRefsVisitor<'a, 'm> {
            builder: &'a mut ComponentBuilder<'m>,
        }

        impl<'a, 'de, 'm> Visitor<'de> for ComponentRefsVisitor<'a, 'm> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a list of component references")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                loop {
                    let component_ref_deserializer =
                        ComponentRefDeserializer::new(self.builder, None);

                    if !seq.next_element_seed(component_ref_deserializer)?.is_some() {
                        break;
                    }
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(ComponentRefsVisitor {
            builder: self.builder,
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
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentNamedRefsVisitor<'a, 'm> {
            builder: &'a mut ComponentBuilder<'m>,
        }

        impl<'a, 'de, 'm> Visitor<'de> for ComponentNamedRefsVisitor<'a, 'm> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a list of named component references")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while let Some(alias) = map.next_key()? {
                    let component_ref_deserializer =
                        ComponentRefDeserializer::new(self.builder, Some(alias));
                    map.next_value_seed(component_ref_deserializer)?;
                }

                Ok(())
            }
        }

        deserializer.deserialize_map(ComponentNamedRefsVisitor {
            builder: self.builder,
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
    type Value = (
        ComponentId,
        HashSet<ComponentWeakRef>,
        HashMap<StringId, ComponentWeakRef>,
    );

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
            type Value = (
                ComponentId,
                HashSet<ComponentWeakRef>,
                HashMap<StringId, ComponentWeakRef>,
            );

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

                while let Some(key) = map.next_key()? {
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
                            map.next_value_seed(ComponentRefsDeserializer::new(&mut builder))?;
                        }
                        Field::NamedReferences => {
                            if !builder.is_unresolved_named_references_empty() {
                                return Err(de::Error::duplicate_field("named references"));
                            }
                            map.next_value_seed(ComponentNamedRefsDeserializer::new(&mut builder))?;
                        }
                        Field::Class => {
                            if builder.is_class_set() {
                                return Err(de::Error::duplicate_field("class"));
                            }
                            builder.set_class(map.next_value()?);
                        }
                    }
                }

                builder
                    .finish()
                    .map_err(|err| de::Error::custom(format!("{err}")))
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
    type Value = HashMap<
        ComponentId,
        (
            HashSet<ComponentWeakRef>,
            HashMap<StringId, ComponentWeakRef>,
        ),
    >;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentsVisitor<'m> {
            module: &'m mut Module,
        }

        impl<'de, 'm> Visitor<'de> for ComponentsVisitor<'m> {
            type Value = HashMap<
                ComponentId,
                (
                    HashSet<ComponentWeakRef>,
                    HashMap<StringId, ComponentWeakRef>,
                ),
            >;

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
