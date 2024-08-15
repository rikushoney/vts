//! Yosys JSON netlist serialization.
//!
//! References:
//! - https://yosyshq.readthedocs.io/projects/yosys/en/latest/cmd/write_json.html
//! - https://github.com/YosysHQ/yosys/blob/1eaf4e07/backends/json/json.cc

use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::fmt;
use std::fs;
use std::io::{BufReader, Read};
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Read(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Design {
    pub creator: String,
    pub modules: FnvHashMap<String, Module>,
}

impl Design {
    pub fn from_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = fs::File::open(path)?;
        Self::from_reader(BufReader::new(file))
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }

    pub fn from_reader<R>(reader: R) -> Result<Self>
    where
        R: Read,
    {
        Ok(serde_json::from_reader(reader)?)
    }
}

impl FromStr for Design {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Module {
    pub attributes: FnvHashMap<String, String>,
    #[serde(default)]
    pub parameter_default_values: FnvHashMap<String, String>,
    pub ports: FnvHashMap<String, Port>,
    pub cells: FnvHashMap<String, Cell>,
    #[serde(default)]
    pub memories: FnvHashMap<String, Memory>,
    pub netnames: FnvHashMap<String, NetName>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Port {
    pub direction: PortDirection,
    pub bits: Vec<usize>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub upto: usize,
    #[serde(default)]
    pub signed: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PortDirection {
    Input,
    Output,
    InOut,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SignalBit {
    Ref(usize),
    Const(ConstBit),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConstBit {
    #[serde(rename = "0")]
    _0,
    #[serde(rename = "1")]
    _1,
    X,
    Z,
}

impl fmt::Display for ConstBit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            ConstBit::_0 => "0",
            ConstBit::_1 => "1",
            ConstBit::X => "x",
            ConstBit::Z => "z",
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Cell {
    pub hide_name: usize,
    #[serde(rename = "type")]
    pub ty: String,
    pub parameters: FnvHashMap<String, String>,
    pub attributes: FnvHashMap<String, String>,
    #[serde(default)]
    pub port_directions: FnvHashMap<String, PortDirection>,
    pub connections: FnvHashMap<String, Vec<SignalBit>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Memory {
    pub hide_name: usize,
    pub attributes: FnvHashMap<String, String>,
    pub width: usize,
    pub start_offset: usize,
    pub size: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NetName {
    pub hide_name: usize,
    pub attributes: FnvHashMap<String, String>,
    pub bits: Vec<SignalBit>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub upto: usize,
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
                        SignalBit::Const(ConstBit::_0),
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
        let parsed: NetName = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            NetName {
                hide_name: 0,
                bits: vec![
                    SignalBit::Ref(2),
                    SignalBit::Const(ConstBit::_0),
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
