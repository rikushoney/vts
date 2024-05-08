use std::collections::HashMap;
use std::ops::Range;
use std::slice;

use thiserror::Error;

use super::{
    ops::{AnyOp, ConstOp},
    yosys::{self, ConstBit, PortDirection, SignalBit},
};

#[derive(Clone, Copy, Debug)]
pub enum NodeKind {
    Source,
    Sink,
    Gate(AnyOp),
}

impl<Op> From<Op> for NodeKind
where
    AnyOp: From<Op>,
{
    fn from(op: Op) -> Self {
        Self::Gate(AnyOp::from(op))
    }
}

#[derive(Clone, Debug)]
pub struct NodeData {
    pub kind: NodeKind,
}

impl NodeData {
    pub fn new_op<Op>(op: Op) -> Self
    where
        AnyOp: From<Op>,
    {
        Self {
            kind: NodeKind::from(op),
        }
    }

    pub fn new_source() -> Self {
        Self {
            kind: NodeKind::Source,
        }
    }

    pub fn new_sink() -> Self {
        Self {
            kind: NodeKind::Sink,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    source: usize,
    sink: usize,
}

#[derive(Clone, Debug)]
struct NodeEntry {
    data: NodeData,
    sinks: Vec<usize>,
}

impl From<NodeData> for NodeEntry {
    fn from(data: NodeData) -> Self {
        Self {
            data,
            sinks: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    entries: Vec<NodeEntry>,
}

impl Graph {
    pub fn new<Ns, Es>(nodes: Ns, edges: Es) -> Self
    where
        Ns: IntoIterator<Item = NodeData>,
        Es: IntoIterator<Item = Edge>,
    {
        let mut graph = Self {
            entries: nodes.into_iter().map(NodeEntry::from).collect(),
        };
        for e in edges.into_iter() {
            graph.add_edge(e);
        }
        graph
    }

    pub fn add_node(&mut self, node: NodeData) -> usize {
        self.entries.push(NodeEntry::from(node));
        self.entries.len() - 1
    }

    pub fn add_nodes<Ns>(&mut self, nodes: Ns) -> Range<usize>
    where
        Ns: IntoIterator<Item = NodeData>,
    {
        let start = self.entries.len();
        for node in nodes.into_iter() {
            self.add_node(node);
        }
        let end = self.entries.len();
        Range { start, end }
    }

    pub fn add_source(&mut self, width: usize) -> Range<usize> {
        self.add_nodes((0..width).map(|_| NodeData::new_source()))
    }

    pub fn add_sink(&mut self, width: usize) -> Range<usize> {
        self.add_nodes((0..width).map(|_| NodeData::new_sink()))
    }

    pub fn add_const(&mut self, k: ConstOp) -> usize {
        self.add_node(NodeData::new_op(k))
    }

    pub fn add_unit(&mut self) -> usize {
        self.add_const(ConstOp::Unit)
    }

    pub fn add_zero(&mut self) -> usize {
        self.add_const(ConstOp::Unit)
    }

    fn check_node(&self, node: usize) {
        assert!(node < self.entries.len(), r#"node {node} out of bounds"#);
    }

    fn check_edge(&self, edge: &Edge) {
        self.check_node(edge.source);
        self.check_node(edge.sink);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.check_edge(&edge);
        self.add_edge_unchecked(edge)
    }

    pub fn add_edge_unchecked(&mut self, edge: Edge) {
        self.entries[edge.source].sinks.push(edge.sink);
    }

    pub fn add_edges<Es>(&mut self, edges: Es)
    where
        Es: IntoIterator<Item = Edge>,
    {
        for e in edges {
            self.add_edge(e);
        }
    }

    pub fn nodes(&self) -> Nodes<'_> {
        Nodes {
            iter: self.entries.iter(),
        }
    }

    pub fn edges(&self) -> Edges<'_> {
        Edges {
            nodes: self.nodes(),
            last_node: 0,
            current: None,
        }
    }
}

pub struct Nodes<'a> {
    iter: slice::Iter<'a, NodeEntry>,
}

impl<'a> Iterator for Nodes<'a> {
    type Item = &'a NodeData;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|entry| &entry.data)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

struct CurrentNode<'a> {
    id: usize,
    sinks: slice::Iter<'a, usize>,
}

pub struct Edges<'a> {
    nodes: Nodes<'a>,
    last_node: usize,
    current: Option<CurrentNode<'a>>,
}

impl<'a> Iterator for Edges<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let entries = &mut self.nodes.iter;
        if self.current.is_none() {
            self.current = entries.next().map(|entry| {
                let n = CurrentNode {
                    id: self.last_node,
                    sinks: entry.sinks.iter(),
                };
                self.last_node += 1;
                n
            });
        }
        if let Some(current) = &mut self.current {
            if let Some(sink) = current.sinks.next() {
                return Some((current.id, *sink));
            } else {
                self.current = None;
            }
        }
        None
    }
}

#[derive(Clone, Debug, Error)]
pub enum YosysError {
    #[error(r#""{0}" not supported"#)]
    Unsupported(String),
    #[error(r#"cell "{0}" should have output Y"#)]
    ShouldHaveOutput(String),
    #[error(r#"multi-bit output ports are not supported ("{0}".Y)"#)]
    MultiBitOutput(String),
    #[error(r#"expected output port to have single bit ("{0}".Y)"#)]
    MissingOutput(String),
    #[error(r#"unexpected const output ("{0}".Y)"#)]
    ConstOutput(String),
    #[error(r#"cell "{cell}" should have input "{port}""#)]
    ShouldHaveInput { cell: String, port: String },
    #[error(r#"multi-bit input ports are not supported ("{cell}".{port})"#)]
    MultiBitInput { cell: String, port: String },
    #[error(r#"expected input port to have single bit ("{cell}".{port})"#)]
    MissingInput { cell: String, port: String },
    #[error(r#"bit {0} out of bounds"#)]
    BitOutOfBounds(usize),
}

impl TryFrom<yosys::Module> for Graph {
    type Error = YosysError;

    fn try_from(module: yosys::Module) -> Result<Self, Self::Error> {
        let mut graph = Self::default();
        let mut bits_to_nodes = HashMap::with_capacity(module.ports.len());
        for port in module.ports.values() {
            let nodes = match port.direction {
                PortDirection::Input => graph.add_source(port.bits.len()),
                PortDirection::Output => graph.add_sink(port.bits.len()),
                PortDirection::InOut => {
                    return Err(YosysError::Unsupported("inout ports".to_string()));
                }
            };
            for (bit, node) in port.bits.iter().zip(nodes) {
                bits_to_nodes.insert(*bit, node);
            }
        }
        let known_cells: Vec<_> = module
            .cells
            .iter()
            .filter_map(|(name, cell)| {
                Some((
                    name,
                    cell,
                    match cell.ty.as_str() {
                        "$_NOT_" => AnyOp::not(),
                        "$_AND_" => AnyOp::and(),
                        "$_OR_" => AnyOp::or(),
                        "$_XOR_" => AnyOp::xor(),
                        "$_MUX_" => AnyOp::mux(),
                        _ => {
                            return None;
                        }
                    },
                ))
            })
            .collect();
        for (name, cell, op) in known_cells {
            let id = graph.add_node(match op {
                AnyOp::Unary(op) => NodeData::new_op(op),
                AnyOp::Binary(op) => NodeData::new_op(op),
                AnyOp::Const(op) => NodeData::new_op(op),
                AnyOp::Mux => NodeData::new_op(op),
            });
            let output = cell
                .connections
                .get("Y")
                .ok_or(YosysError::ShouldHaveOutput(cell.ty.clone()))
                .and_then(|output| {
                    let mut bits = output.iter();
                    if let Some(bit) = bits.next() {
                        let bit = match bit {
                            SignalBit::Ref(bit) => *bit,
                            SignalBit::Const(_) => {
                                return Err(YosysError::ConstOutput(name.clone()))
                            }
                        };
                        if bits.next().is_none() {
                            Ok(bit)
                        } else {
                            Err(YosysError::MultiBitOutput(name.clone()))
                        }
                    } else {
                        Err(YosysError::MissingOutput(name.clone()))
                    }
                })?;
            bits_to_nodes.insert(output, id);
            let input_a = cell
                .connections
                .get("A")
                .ok_or(YosysError::ShouldHaveInput {
                    cell: cell.ty.clone(),
                    port: "A".to_string(),
                })
                .and_then(|input| {
                    let mut bits = input.iter();
                    if let Some(bit) = bits.next() {
                        if bits.next().is_none() {
                            Ok(bit)
                        } else {
                            Err(YosysError::MultiBitInput {
                                cell: name.clone(),
                                port: "A".to_string(),
                            })
                        }
                    } else {
                        Err(YosysError::MissingInput {
                            cell: name.clone(),
                            port: "A".to_string(),
                        })
                    }
                })?;
            match input_a {
                SignalBit::Ref(_bit) => {
                    // TODO: add edge to graph
                }
                SignalBit::Const(bit) => {
                    let node = match bit {
                        ConstBit::Zero => graph.add_zero(),
                        ConstBit::One => graph.add_unit(),
                        k => {
                            return Err(YosysError::Unsupported(format!("{k} constants")));
                        }
                    };
                    // TODO: add edge to graph
                }
            }
            if matches!(op, AnyOp::Binary(_) | AnyOp::Mux) {
                let input_b = cell
                    .connections
                    .get("B")
                    .ok_or(YosysError::ShouldHaveInput {
                        cell: cell.ty.clone(),
                        port: "B".to_string(),
                    })
                    .and_then(|input| {
                        let mut bits = input.iter();
                        if let Some(bit) = bits.next() {
                            if bits.next().is_none() {
                                Ok(bit)
                            } else {
                                Err(YosysError::MultiBitInput {
                                    cell: name.clone(),
                                    port: "B".to_string(),
                                })
                            }
                        } else {
                            Err(YosysError::MissingInput {
                                cell: name.clone(),
                                port: "B".to_string(),
                            })
                        }
                    })?;
                match input_b {
                    SignalBit::Ref(_bit) => {
                        // TODO: add edge to graph
                    }
                    SignalBit::Const(bit) => {
                        let node = match bit {
                            ConstBit::Zero => graph.add_zero(),
                            ConstBit::One => graph.add_unit(),
                            k => {
                                return Err(YosysError::Unsupported(format!("{k} constants")));
                            }
                        };
                        // TODO: add edge to graph
                    }
                }
            }
            if matches!(op, AnyOp::Mux) {
                let input_s = cell
                    .connections
                    .get("S")
                    .ok_or(YosysError::ShouldHaveInput {
                        cell: cell.ty.clone(),
                        port: "S".to_string(),
                    })
                    .and_then(|input| {
                        let mut bits = input.iter();
                        if let Some(bit) = bits.next() {
                            if let None = bits.next() {
                                Ok(bit)
                            } else {
                                Err(YosysError::MultiBitInput {
                                    cell: name.clone(),
                                    port: "S".to_string(),
                                })
                            }
                        } else {
                            Err(YosysError::MissingInput {
                                cell: name.clone(),
                                port: "S".to_string(),
                            })
                        }
                    })?;
                match input_s {
                    SignalBit::Ref(_bit) => {
                        // TODO: add edge to graph
                    }
                    SignalBit::Const(bit) => {
                        let node = match bit {
                            ConstBit::Zero => graph.add_zero(),
                            ConstBit::One => graph.add_unit(),
                            k => {
                                return Err(YosysError::Unsupported(format!("{k} constants")));
                            }
                        };
                        // TODO: add edge to graph
                    }
                }
            }
        }
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ops::BinaryOp;

    macro_rules! binop {
        (and) => {
            NodeData::new_op(BinaryOp::And)
        };
        (or) => {
            NodeData::new_op(BinaryOp::Or)
        };
        (xor) => {
            NodeData::new_op(BinaryOp::Xor)
        };
    }

    macro_rules! src {
        () => {
            NodeData::new_source()
        };
        ($count:literal) => {
            vec![NodeData::new_source(); $count]
        };
    }

    macro_rules! sink {
        () => {
            NodeData::new_sink()
        };
        ($count:literal) => {
            vec![NodeData::new_sink(); $count]
        };
    }

    macro_rules! edge {
        ($src:literal : $sink:literal) => {
            Edge {
                source: $src,
                sink: $sink,
            }
        };
    }

    macro_rules! edges {
        ($($src:literal : $sink:literal),+ $(,)?) => {
            vec![
                $(
                    edge!($src : $sink),
                )+
            ]
        }
    }

    fn get_test_graph() -> Graph {
        let nodes: Vec<_> = vec![
            // A 0-3
            src!(4),
            // B 4-7
            src!(4),
            vec![
                // Cin 8
                src!(),
                // Gates 9-28
                binop!(xor),
                binop!(and),
                binop!(and),
                binop!(or),
                binop!(xor),
                binop!(and),
                binop!(and),
                binop!(or),
                binop!(xor),
                binop!(and),
                binop!(and),
                binop!(or),
                binop!(xor),
                binop!(xor),
                binop!(xor),
                binop!(xor),
                binop!(xor),
                binop!(and),
                binop!(and),
                binop!(or),
                // Cout 29
                sink!(),
            ],
            // S 30-33
            sink!(4),
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(nodes.len(), 34);
        let edges = edges![
            0:9,
            0:11,
            1:13,
            1:14,
            2:17,
            2:19,
            3:21,
            3:26,
            4:9,
            4:11,
            5:13,
            5:14,
            6:17,
            6:19,
            7:21,
            7:26,
            8:10,
            8:22,
            9:10,
            9:22,
            10:12,
            11:12,
            12:15,
            12:23,
            13:15,
            13:23,
            14:16,
            15:16,
            16:18,
            16:24,
            17:18,
            17:24,
            18:20,
            19:20,
            20:25,
            20:27,
            21:25,
            21:27,
            22:30,
            23:31,
            24:32,
            25:33,
            26:28,
            27:28,
            28:29,
        ];
        assert_eq!(edges.len(), 45);
        Graph::new(nodes, edges)
    }

    #[test]
    fn test_new_graph() {
        get_test_graph();
    }
}
