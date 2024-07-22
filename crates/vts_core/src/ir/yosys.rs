//! Yosys JSON netlist format
//!
//! # Usage
//! The top-most data type is a [`Netlist`] which is read from
//! a [JSON](https://www.json.org/) file. There are various
//! `from` methods available to read a JSON file into a netlist.
//! See [`Netlist::from_str`], [`Netlist::from_file`], [`Netlist::from_slice`]
//! and [`Netlist::from_reader`].
//!
//! Reference: <https://yosyshq.readthedocs.io/projects/yosys/en/latest/cmd/write_json.html>

use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};

use std::{error, fmt, fs, io::Read, path::Path, str::FromStr};

/// A structural description of a circuit
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Netlist {
    /// The program that created the netlist
    pub creator: String,
    /// A mapping from module names to [`Module`] instances
    pub modules: FnvHashMap<String, Module>,
}

impl Netlist {
    /// Read a netlist from a file
    pub fn from_file<P>(path: P) -> Result<Self, Box<dyn error::Error>>
    where
        P: AsRef<Path>,
    {
        let json = fs::read_to_string(path)?;
        Self::from_str(json.as_str()).map_err(Box::from)
    }

    /// Read a netlist from the contents of a byte slice
    pub fn from_slice(s: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(s)
    }

    /// Read a netlist using a reader
    pub fn from_reader<R>(r: R) -> serde_json::Result<Self>
    where
        R: Read,
    {
        serde_json::from_reader(r)
    }
}

impl FromStr for Netlist {
    type Err = serde_json::Error;

    /// Read a netlist from a string
    fn from_str(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}

/// A design unit encapsulating ports, cells, memories and wires
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Module {
    /// Module attributes
    pub attributes: FnvHashMap<String, String>,
    /// Default parameter values
    #[serde(default)]
    pub parameter_default_values: FnvHashMap<String, String>,
    /// Module ports
    pub ports: FnvHashMap<String, Port>,
    /// Module cells
    pub cells: FnvHashMap<String, Cell>,
    /// Module memories
    #[serde(default)]
    pub memories: FnvHashMap<String, Memory>,
    /// Module net names
    pub netnames: FnvHashMap<String, Netname>,
}

/// A connection point for wires that is either an input, output or both
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Port {
    /// Port direction
    pub direction: PortDirection,
    /// The signal "bits" of the port
    pub bits: Vec<usize>,
    /// The lowest bit index of the port
    #[serde(default)]
    pub offset: usize,
    /// 1 if indexing starts at the MSB, otherwise 0
    #[serde(default)]
    pub upto: usize,
    /// 1 if the port is signed, otherwise 0
    #[serde(default)]
    pub signed: usize,
}

/// Indicates the direction of a [`Port`]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PortDirection {
    Input,
    Output,
    InOut,
}

/// A reference to a single wire "bit" or a constant value
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SignalBit {
    /// A reference to a numbered wire
    Ref(usize),
    /// A constant value
    Const(ConstBit),
}

/// The possible states of a wire
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConstBit {
    #[serde(rename = "0")]
    /// Logic "low"
    Zero,
    #[serde(rename = "1")]
    /// Logic "high"
    One,
    /// Unknown/invalid
    X,
    /// High impedance
    Z,
}

impl fmt::Display for ConstBit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ConstBit::Zero => "0",
                ConstBit::One => "1",
                ConstBit::X => "x",
                ConstBit::Z => "z",
            }
        )
    }
}

/// A basic building block of a circuit such as logic gates or registers
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Cell {
    /// 1 if the name of the cell is hidden, otherwise 0
    pub hide_name: usize,
    /// The type of cell
    #[serde(rename = "type")]
    pub ty: String,
    /// Cell parameters
    pub parameters: FnvHashMap<String, String>,
    /// Cell attributes
    pub attributes: FnvHashMap<String, String>,
    /// The directions of the cell's ports
    #[serde(default)]
    pub port_directions: FnvHashMap<String, PortDirection>,
    /// The signal bits connected to the cell
    pub connections: FnvHashMap<String, Vec<SignalBit>>,
}

/// A block of memory
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Memory {
    /// 1 if the name of the memory is hidden, otherwise 0
    pub hide_name: usize,
    /// Memory attributes
    pub attributes: FnvHashMap<String, String>,
    /// The memory word size
    pub width: usize,
    /// The starting index offset
    pub start_offset: usize,
    /// The number of words in memory
    pub size: usize,
}

/// The name given to a net in a circuit
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Netname {
    /// 1 if the net name is hidden, otherwise 0
    pub hide_name: usize,
    /// Net name attributes
    pub attributes: FnvHashMap<String, String>,
    /// The signal "bits" of the net
    pub bits: Vec<SignalBit>,
    /// The lowest bit index
    #[serde(default)]
    pub offset: usize,
    /// 1 if indexing starts at the MSB, otherwise 0
    #[serde(default)]
    pub upto: usize,
    /// 1 if the net is signed, otherwise 0
    #[serde(default)]
    pub signed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_port_details() {
        let json = r#"{"direction": "input", "bits": [0, 1, 2, 3]}"#;
        let parsed: Port = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            Port {
                direction: PortDirection::Input,
                bits: vec![0, 1, 2, 3],
                offset: 0,
                upto: 0,
                signed: 0
            }
        );
    }

    #[test]
    fn test_parse_cell_details() {
        let json = r#"{
    "hide_name": 0,
    "type": "test_cell",
    "parameters": {
        "A_SIGNED": "00000000000000000000000000000001",
        "A_WIDTH": "00000000000000000000000000000100"
    },
    "attributes": {
        "src": "test.v"
    },
    "port_directions": {
        "A": "output"
    },
    "connections": {
        "A": [4, "0", "x", 5]
    }
}"#;
        let parsed: Cell = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            Cell {
                hide_name: 0,
                ty: "test_cell".to_string(),
                parameters: FnvHashMap::from_iter([
                    (
                        "A_SIGNED".to_string(),
                        "00000000000000000000000000000001".to_string()
                    ),
                    (
                        "A_WIDTH".to_string(),
                        "00000000000000000000000000000100".to_string()
                    )
                ]),
                attributes: FnvHashMap::from_iter([("src".to_string(), "test.v".to_string())]),
                port_directions: FnvHashMap::from_iter([("A".to_string(), PortDirection::Output)]),
                connections: FnvHashMap::from_iter([(
                    "A".to_string(),
                    vec![
                        SignalBit::Ref(4),
                        SignalBit::Const(ConstBit::Zero),
                        SignalBit::Const(ConstBit::X),
                        SignalBit::Ref(5)
                    ]
                )])
            }
        );
    }

    #[test]
    fn test_parse_memory_details() {
        let json = r#"{
    "hide_name": 1,
    "attributes": {
        "src": "test.v"
    },
    "width": 32,
    "start_offset": 1024,
    "size": 8192
}"#;
        let parsed: Memory = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            Memory {
                hide_name: 1,
                attributes: FnvHashMap::from_iter([("src".to_string(), "test.v".to_string())]),
                width: 32,
                start_offset: 1024,
                size: 8192
            }
        );
    }

    #[test]
    fn test_parse_net_details() {
        let json = r#"{
    "hide_name": 0,
    "attributes": {
        "src": "test.v"
    },
    "bits": [2, "0", 3, "x"]
}"#;
        let parsed: Netname = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            Netname {
                hide_name: 0,
                bits: vec![
                    SignalBit::Ref(2),
                    SignalBit::Const(ConstBit::Zero),
                    SignalBit::Ref(3),
                    SignalBit::Const(ConstBit::X)
                ],
                attributes: FnvHashMap::from_iter([("src".to_string(), "test.v".to_string())]),
                offset: 0,
                upto: 0,
                signed: 0
            }
        )
    }
}
