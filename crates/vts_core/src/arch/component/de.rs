use std::collections::HashMap;
use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::arch::{
    component::{ComponentBuildArtifacts, ComponentBuilder, WeakConnectionBuilder},
    port::de::PortsDeserializer,
    ComponentId, Module,
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

                    if seq.next_element_seed(component_ref_deserializer)?.is_none() {
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

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ConnectionFields {
    Source,
    Sink,
}

struct InterfaceDeserializer<'a, 'b, 'm> {
    source_or_sink: ConnectionFields,
    builder: &'a mut WeakConnectionBuilder<'b, 'm>,
}

impl<'a, 'b, 'de, 'm> DeserializeSeed<'de> for InterfaceDeserializer<'a, 'b, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct InterfaceVisitor<'a, 'b, 'm> {
            source_or_sink: ConnectionFields,
            builder: &'a mut WeakConnectionBuilder<'b, 'm>,
        }

        impl<'a, 'b, 'de, 'm> Visitor<'de> for InterfaceVisitor<'a, 'b, 'm> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an interface description")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "lowercase")]
                enum Fields {
                    Port,
                    #[serde(rename = "port_start")]
                    PortStart,
                    #[serde(rename = "port_end")]
                    PortEnd,
                    Component,
                }

                let mut port: Option<String> = None;
                let mut port_start: Option<u32> = None;
                let mut port_end: Option<u32> = None;
                let mut component: Option<String> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Fields::Port => {
                            if port.is_some() {
                                return Err(de::Error::duplicate_field("port"));
                            }
                            port = Some(map.next_value()?);
                        }
                        Fields::PortStart => {
                            if port_start.is_some() {
                                return Err(de::Error::duplicate_field("start"));
                            }
                            port_start = Some(map.next_value()?);
                        }
                        Fields::PortEnd => {
                            if port_end.is_some() {
                                return Err(de::Error::duplicate_field("end"));
                            }
                            port_end = Some(map.next_value()?);
                        }
                        Fields::Component => {
                            if component.is_some() {
                                return Err(de::Error::duplicate_field("component"));
                            }
                            component = Some(map.next_value()?);
                        }
                    }
                }

                let port = port.ok_or(de::Error::missing_field("port"))?;
                let port_start = port_start.ok_or(de::Error::missing_field("port_start"))?;
                let port_end = port_end.ok_or(de::Error::missing_field("port_end"))?;
                let component = component.as_deref();

                match self.source_or_sink {
                    ConnectionFields::Source => {
                        self.builder
                            .set_source(port.as_str(), port_start..port_end, component);
                    }
                    ConnectionFields::Sink => {
                        self.builder
                            .set_sink(port.as_str(), port_start..port_end, component);
                    }
                }

                Ok(())
            }
        }

        deserializer.deserialize_map(InterfaceVisitor {
            source_or_sink: self.source_or_sink,
            builder: self.builder,
        })
    }
}

struct ConnectionDeserializer<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for ConnectionDeserializer<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ConnectionVisitor<'a, 'm> {
            builder: &'a mut ComponentBuilder<'m>,
        }

        const FIELDS: &[&str] = &["source", "sink"];

        impl<'a, 'de, 'm> Visitor<'de> for ConnectionVisitor<'a, 'm> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a connection description")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut builder = self.builder.add_weak_connection();

                while let Some(key) = map.next_key()? {
                    match key {
                        ConnectionFields::Source => {
                            if builder.is_source_set() {
                                return Err(de::Error::duplicate_field("source"));
                            }

                            let interface_deserializer = InterfaceDeserializer {
                                source_or_sink: ConnectionFields::Source,
                                builder: &mut builder,
                            };
                            map.next_value_seed(interface_deserializer)?;
                        }
                        ConnectionFields::Sink => {
                            if builder.is_sink_set() {
                                return Err(de::Error::duplicate_field("sink"));
                            }

                            let interface_deserializer = InterfaceDeserializer {
                                source_or_sink: ConnectionFields::Sink,
                                builder: &mut builder,
                            };
                            map.next_value_seed(interface_deserializer)?;
                        }
                    }
                }

                builder
                    .finish()
                    .map_err(|err| de::Error::custom(format!("{err}")))?;

                Ok(())
            }
        }

        deserializer.deserialize_struct(
            "Connection",
            FIELDS,
            ConnectionVisitor {
                builder: self.builder,
            },
        )
    }
}

struct ConnectionsDeserializer<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
}

impl<'a, 'm> ConnectionsDeserializer<'a, 'm> {
    fn new(builder: &'a mut ComponentBuilder<'m>) -> Self {
        Self { builder }
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for ConnectionsDeserializer<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ConnectionsVisitor<'a, 'm> {
            builder: &'a mut ComponentBuilder<'m>,
        }

        impl<'a, 'de, 'm> Visitor<'de> for ConnectionsVisitor<'a, 'm> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a list of connections")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                loop {
                    let connection_deserializer = ConnectionDeserializer {
                        builder: self.builder,
                    };

                    if seq.next_element_seed(connection_deserializer)?.is_none() {
                        break;
                    }
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(ConnectionsVisitor {
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
    type Value = (ComponentId, ComponentBuildArtifacts);

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
            type Value = (ComponentId, ComponentBuildArtifacts);

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
                    Connections,
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
                        Field::Connections => {
                            if !builder.is_connections_empty() {
                                return Err(de::Error::duplicate_field("connections"));
                            }
                            map.next_value_seed(ConnectionsDeserializer::new(&mut builder))?;
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
    type Value = HashMap<ComponentId, ComponentBuildArtifacts>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ComponentsVisitor<'m> {
            module: &'m mut Module,
        }

        impl<'de, 'm> Visitor<'de> for ComponentsVisitor<'m> {
            type Value = HashMap<ComponentId, ComponentBuildArtifacts>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of component descriptions")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut component_artifacts = match map.size_hint() {
                    Some(count) => HashMap::with_capacity(count),
                    None => HashMap::default(),
                };

                while let Some(name) = map.next_key()? {
                    let (component, artifacts) =
                        map.next_value_seed(ComponentDeserializer::new(self.module, name))?;

                    component_artifacts.insert(component, artifacts);
                }

                Ok(component_artifacts)
            }
        }

        deserializer.deserialize_map(ComponentsVisitor {
            module: self.module,
        })
    }
}
