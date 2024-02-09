use crate::model::{self, BlockClass, ConnectionKind, PinRange, PortClass, PortDirection};

use fnv::FnvHashMap as HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Architecture<'a> {
    pub blocks: Vec<Block<'a>>,
}

impl<'a> Architecture<'a> {
    pub fn make_concrete(&self) -> Result<model::Architecture, ()> {
        let mut blocks = HashMap::default();
        for block in self.blocks.iter() {
            blocks.insert(block.name.to_string(), block.make_concrete()?);
        }
        Ok(model::Architecture { blocks })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block<'a> {
    pub name: &'a str,
    pub count: usize,
    pub children: Vec<Block<'a>>,
    pub ports: Vec<Port<'a>>,
    pub connections: Vec<Connection<'a>>,
    pub class: Option<BlockClass>,
}

impl<'a> Block<'a> {
    pub fn make_concrete(&self) -> Result<model::Block, ()> {
        let mut children = HashMap::default();
        for block in self.children.iter() {
            children.insert(block.name.to_string(), block.make_concrete()?);
        }
        let mut ports = HashMap::default();
        for port in self.ports.iter() {
            ports.insert(
                port.name.to_string(),
                model::Port {
                    pins: port.pins,
                    direction: port.direction,
                    class: port.class,
                },
            );
        }
        let mut connections = vec![];
        for conn in self.connections.iter() {
            let mut input_spec = vec![];
            for input in conn.input.iter() {
                input_spec.push((input.0.name.to_string(), input.1));
            }
            let mut output_spec = vec![];
            for output in conn.output.iter() {
                output_spec.push((output.0.name.to_string(), output.1));
            }
            connections.push(model::Connection {
                input: input_spec,
                output: output_spec,
                kind: conn.kind,
            });
        }
        Ok(model::Block {
            count: self.count,
            children,
            ports,
            connections,
            class: self.class,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port<'a> {
    pub name: &'a str,
    pub pins: usize,
    pub direction: PortDirection,
    pub class: Option<PortClass>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Connection<'a> {
    pub input: Vec<(&'a Port<'a>, PinRange)>,
    pub output: Vec<(&'a Port<'a>, PinRange)>,
    pub kind: ConnectionKind,
}
