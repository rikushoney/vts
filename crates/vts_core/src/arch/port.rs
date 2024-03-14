use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use crate::arch::{Component, StringId};

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

#[derive(Clone, Debug, PartialEq)]
pub struct Port<'m> {
    parent: &'m Component<'m>,
    name: StringId,
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}

impl<'m> Port<'m> {
    pub(crate) fn new(
        parent: &'m Component,
        name: StringId,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> Self {
        Self {
            parent,
            name,
            kind,
            n_pins,
            class,
        }
    }

    pub fn name(&self) -> &str {
        self.parent.module.strings.lookup(self.name)
    }
}

impl<'m> Serialize for Port<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut port_serializer = serializer.serialize_struct("Port", 4)?;

        let name = self.parent.module.strings.lookup(self.name);
        port_serializer.serialize_field("name", name)?;

        port_serializer.serialize_field("kind", &self.kind)?;
        port_serializer.serialize_field("n_pins", &self.n_pins)?;
        port_serializer.serialize_field("class", &self.class)?;

        port_serializer.end()
    }
}
