use fnv::FnvHashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Netlist {
    pub creator: String,
    pub modules: FnvHashMap<String, Module>,
}

#[derive(Deserialize)]
pub struct Module {
    pub attributes: FnvHashMap<String, String>,
    #[serde(default)]
    pub parameter_default_values: FnvHashMap<String, String>,
    pub ports: FnvHashMap<String, Port>,
    pub cells: FnvHashMap<String, Cell>,
    #[serde(default)]
    pub memories: FnvHashMap<String, Memory>,
    pub netnames: FnvHashMap<String, Netname>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Port {
    pub direction: PortDirection,
    pub bits: Vec<SignalBit>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub upto: usize,
    #[serde(default)]
    pub signed: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PortDirection {
    Input,
    Output,
    InOut,
}

pub type SignalRef = usize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SignalBit {
    Ref(SignalRef),
    Const(ConstBit),
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConstBit {
    #[serde(rename = "0")]
    Zero,
    #[serde(rename = "1")]
    One,
    X,
    Z,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Cell {
    pub hide_name: usize,
    #[serde(rename = "type")]
    pub ty: String,
    pub model: String,
    pub parameters: FnvHashMap<String, String>,
    pub attributes: FnvHashMap<String, String>,
    #[serde(default)]
    pub port_directions: FnvHashMap<String, PortDirection>,
    pub connections: FnvHashMap<String, Vec<SignalBit>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Memory {
    pub hide_name: usize,
    pub attributes: FnvHashMap<String, String>,
    pub width: usize,
    pub start_offset: usize,
    pub size: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Netname {
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
        let json = r#"{"direction": "input", "bits": ["x", 3, "1", 1]}"#;
        let parsed: Port = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            Port {
                direction: PortDirection::Input,
                bits: vec![
                    SignalBit::Const(ConstBit::X),
                    SignalBit::Ref(3),
                    SignalBit::Const(ConstBit::One),
                    SignalBit::Ref(1)
                ],
                offset: 0,
                upto: 0,
                signed: 0
            }
        );
    }
}
