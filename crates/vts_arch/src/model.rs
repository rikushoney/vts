use crate::architecture as arch;

use fnv::FnvHashMap as HashMap;
use serde::Deserialize;

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Architecture {
    pub blocks: HashMap<String, Block>,
}

impl Architecture {
    pub fn make_abstract(&self) -> Result<arch::Architecture, ()> {
        let mut blocks = vec![];
        for (name, block) in self.blocks.iter() {
            // TODO(rikus): we first need to populate all the blocks,
            // then we can resolve connections
            blocks.push(block.make_abstract_partial(&name)?);
        }
        Ok(arch::Architecture { blocks })
    }
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Block {
    pub count: usize,
    pub children: HashMap<String, Block>,
    pub ports: HashMap<String, Port>,
    pub connections: Vec<Connection>,
    pub class: Option<BlockClass>,
}

impl Block {
    pub fn make_abstract_partial<'a>(&'a self, name: &'a str) -> Result<arch::Block<'a>, ()> {
        let mut children = vec![];
        for (name, block) in self.children.iter() {
            children.push(block.make_abstract_partial(name)?);
        }
        let mut ports = vec![];
        for (name, port) in self.ports.iter() {
            ports.push(arch::Port {
                name,
                pins: port.pins,
                direction: port.direction,
                class: port.class,
            });
        }
        Ok(arch::Block {
            name,
            count: self.count,
            children,
            ports,
            connections: vec![],
            class: self.class,
        })
    }
}

#[derive(Copy, Clone, Deserialize, Debug, PartialEq)]
pub enum BlockClass {
    #[serde(rename = "LUT")]
    LookupTable,
    #[serde(rename = "LATCH")]
    Latch,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Port {
    pub pins: usize,
    pub direction: PortDirection,
    pub class: Option<PortClass>,
}

#[derive(Copy, Clone, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Copy, Clone, Deserialize, Debug, PartialEq)]
pub enum PortClass {
    #[serde(rename = "LUT_IN")]
    LookupTableIn,
    #[serde(rename = "LUT_OUT")]
    LookupTableOut,
    #[serde(rename = "LATCH_IN")]
    LatchIn,
    #[serde(rename = "LATCH_OUT")]
    LatchOut,
    #[serde(rename = "CLK")]
    Clock,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Connection {
    pub input: Vec<(String, PinRange)>,
    pub output: Vec<(String, PinRange)>,
    pub kind: ConnectionKind,
}

#[derive(Copy, Clone, Deserialize, Debug, PartialEq)]
pub enum PinRange {
    Single(usize),
    From(usize),
    Inclusive { start: usize, end: usize },
}

#[derive(Copy, Clone, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConnectionKind {
    Direct,
    Complete,
    Mux,
}
