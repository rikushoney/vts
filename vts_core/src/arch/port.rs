use std::fmt;
use std::ops::Range;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use thiserror::Error;

use super::{component::ComponentKey, Component, ComponentId, Module, PortId};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum PortClass {
    #[serde(rename = "CLOCK")]
    Clock,
    #[serde(rename = "LUT_IN")]
    LutIn,
    #[serde(rename = "LUT_OUT")]
    LutOut,
    #[serde(rename = "LATCH_IN")]
    LatchIn,
    #[serde(rename = "LATCH_OUT")]
    LatchOut,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PortData {
    #[serde(skip)]
    pub name: String,
    #[serde(skip)]
    parent: ComponentId,
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}

impl PortData {
    pub(crate) fn new(
        parent: ComponentId,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> Self {
        Self {
            name: name.to_string(),
            parent,
            kind,
            n_pins,
            class,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PortKey(pub(crate) PortId);

impl PortKey {
    pub(crate) fn new(port: PortId) -> Self {
        Self(port)
    }

    pub fn bind(self, module: &Module) -> Port<'_> {
        Port::new(module, self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Port<'m>(&'m Module, PortId);

impl<'m> Port<'m> {
    pub(crate) fn new(module: &'m Module, port: PortId) -> Self {
        Self(module, port)
    }

    pub fn module(&self) -> &'m Module {
        self.0
    }

    pub fn key(&self) -> PortKey {
        PortKey::new(self.1)
    }

    pub fn parent(&self) -> Component<'_> {
        Component::new(self.0, self.data().parent)
    }

    pub fn name(&self) -> &str {
        &self.module()[self.1].name
    }

    pub(crate) fn data(&self) -> &'m PortData {
        &self.module().ports[self.1]
    }

    pub fn kind(&self) -> PortKind {
        self.data().kind
    }

    pub fn n_pins(&self) -> usize {
        self.data().n_pins
    }

    pub fn class(&self) -> Option<PortClass> {
        self.data().class
    }

    #[must_use]
    pub fn select(&self, range: Range<u32>) -> PortPins {
        PortPins::new(self.1, range)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PortPins {
    port: PortId,
    range: Range<u32>,
}

impl PortPins {
    pub(crate) fn new(port: PortId, range: Range<u32>) -> Self {
        Self { port, range }
    }

    pub fn start(&self) -> u32 {
        self.range.start
    }

    pub fn end(&self) -> u32 {
        self.range.end
    }

    pub fn range(&self) -> Range<u32> {
        self.range.clone()
    }

    pub fn port<'m>(&self, module: &'m Module) -> Port<'m> {
        Port::new(module, self.port)
    }
}

pub struct NameSet(String);
pub struct NameUnset;
pub struct KindSet(PortKind);
pub struct KindUnset;

pub struct PortBuilder<'m, N, K> {
    module: &'m mut Module,
    parent: ComponentId,
    name: N,
    kind: K,
    n_pins: Option<usize>,
    class: Option<PortClass>,
}

impl<'m> PortBuilder<'m, NameUnset, KindUnset> {
    pub fn new(module: &'m mut Module, component: ComponentKey) -> Self {
        Self {
            module,
            parent: component.0,
            name: NameUnset,
            kind: KindUnset,
            n_pins: None,
            class: None,
        }
    }
}

impl<'m, K> PortBuilder<'m, NameUnset, K> {
    pub fn set_name(self, name: &str) -> PortBuilder<'m, NameSet, K> {
        PortBuilder {
            module: self.module,
            parent: self.parent,
            name: NameSet(name.to_string()),
            kind: self.kind,
            n_pins: self.n_pins,
            class: self.class,
        }
    }
}

impl<'m, N> PortBuilder<'m, N, KindUnset> {
    pub fn set_kind(self, kind: PortKind) -> PortBuilder<'m, N, KindSet> {
        PortBuilder {
            module: self.module,
            parent: self.parent,
            name: self.name,
            kind: KindSet(kind),
            n_pins: self.n_pins,
            class: self.class,
        }
    }
}

impl<'m, N, K> PortBuilder<'m, N, K> {
    pub fn set_n_pins(&mut self, n_pins: usize) {
        self.n_pins = Some(n_pins);
    }

    pub fn set_class(&mut self, class: PortClass) {
        self.class = Some(class);
    }

    pub fn n_pins_is_set(&self) -> bool {
        self.n_pins.is_some()
    }

    pub fn class_is_set(&self) -> bool {
        self.class.is_some()
    }
}

impl<'m> PortBuilder<'m, NameSet, KindSet> {
    fn insert(&mut self) -> PortId {
        // TODO: check duplicate ports
        let port = PortData::new(
            self.parent,
            &self.name.0,
            self.kind.0,
            self.n_pins.unwrap_or(1),
            self.class,
        );

        self.module.ports.insert(port)
    }

    pub fn finish(mut self) -> Port<'m> {
        let port = self.insert();
        self.module[self.parent].ports.push(port);
        Port::new(self.module, port)
    }
}

#[derive(Debug, Error)]
pub enum PortBuildError {
    #[error(r#"port "{port}" already in "{module}""#)]
    DuplicatePort { module: String, port: String },
    #[error("port must have a {0}")]
    MissingField(&'static str),
}

pub(crate) struct PortSeed<'m> {
    module: &'m mut Module,
    parent: ComponentId,
    name: String,
}

impl<'m> PortSeed<'m> {
    pub(crate) fn new(module: &'m mut Module, parent: ComponentId, name: String) -> Self {
        Self {
            module,
            parent,
            name,
        }
    }
}

impl<'de, 'm> Visitor<'de> for PortSeed<'m> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a port description")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum Field {
            Kind,
            #[serde(rename = "n_pins")]
            NPins,
            Class,
        }

        let mut kind: Option<PortKind> = None;
        let mut n_pins: Option<usize> = None;
        let mut class: Option<PortClass> = None;

        while let Some(field) = map.next_key()? {
            match field {
                Field::Kind => {
                    if kind.is_some() {
                        return Err(de::Error::duplicate_field("kind"));
                    }

                    kind = Some(map.next_value()?);
                }
                Field::NPins => {
                    if n_pins.is_some() {
                        return Err(de::Error::duplicate_field("n_pins"));
                    }

                    n_pins = Some(map.next_value()?);
                }
                Field::Class => {
                    if class.is_some() {
                        return Err(de::Error::duplicate_field("class"));
                    }

                    class = Some(map.next_value()?);
                }
            }
        }

        let kind = kind.ok_or(de::Error::missing_field("kind"))?;

        let mut builder = PortBuilder::new(self.module, ComponentKey(self.parent))
            .set_name(&self.name)
            .set_kind(kind);

        if let Some(n_pins) = n_pins {
            builder.set_n_pins(n_pins);
        }

        if let Some(class) = class {
            builder.set_class(class);
        }

        builder.finish();
        Ok(())
    }
}

impl<'de, 'm> DeserializeSeed<'de> for PortSeed<'m> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        const FIELDS: &[&str] = &["kind", "n_pins", "class"];
        deserializer.deserialize_struct("Port", FIELDS, self)
    }
}

mod pin_range {
    use super::*;

    const FIELDS: &[&str] = &["port_start", "port_end"];
    const PORT_START: usize = 0;
    const PORT_END: usize = 1;

    pub fn serialize<S: Serializer>(range: &Range<u32>, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("PinRange", FIELDS.len())?;
        state.serialize_field(FIELDS[PORT_START], &(range.start as u32))?;
        state.serialize_field(FIELDS[PORT_END], &(range.end as u32))?;
        state.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Range<u32>, D::Error> {
        struct PinRangeVisitor;

        impl<'de> Visitor<'de> for PinRangeVisitor {
            type Value = Range<u32>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a pin range description")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                enum Field {
                    PortStart,
                    PortEnd,
                }

                let mut start: Option<u32> = None;
                let mut end: Option<u32> = None;

                while let Some(field) = map.next_key()? {
                    match field {
                        Field::PortStart => {
                            if start.is_some() {
                                return Err(de::Error::duplicate_field(FIELDS[PORT_START]));
                            }

                            start = Some(map.next_value()?);
                        }
                        Field::PortEnd => {
                            if end.is_some() {
                                return Err(de::Error::duplicate_field(FIELDS[PORT_END]));
                            }

                            end = Some(map.next_value()?);
                        }
                    }
                }

                let start = start.ok_or(de::Error::missing_field(FIELDS[PORT_START]))?;
                let end = end.ok_or(de::Error::missing_field(FIELDS[PORT_END]))?;

                Ok(Range { start, end })
            }
        }

        deserializer.deserialize_struct("PinRange", FIELDS, PinRangeVisitor)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct WeakPortPins {
    pub port: String,
    #[serde(flatten, with = "pin_range")]
    pub range: Range<u32>,
}
