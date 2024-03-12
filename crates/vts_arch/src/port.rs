use serde::Deserialize;

use crate::{Component, Module, StringId};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
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
