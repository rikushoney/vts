use fnv::FnvHashMap as HashMap;

use serde::Deserialize;

use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Module {
    pub cells: HashMap<Arc<str>, Arc<Cell>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CellClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Cell {
    pub ports: HashMap<Arc<str>, Arc<Port>>,
    pub subcells: HashMap<Arc<str>, Arc<Cell>>,
    pub class: Option<CellClass>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
    // Clock,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
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

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Port {
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}
