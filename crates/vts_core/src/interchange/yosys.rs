//! Yosys JSON netlist serialization.
//!
//! References:
//! - https://yosyshq.readthedocs.io/projects/yosys/en/latest/cmd/write_json.html
//! - https://github.com/YosysHQ/yosys/blob/1eaf4e0/backends/json/json.cc
//! - https://github.com/YosysHQ/yosys/blob/0fc5812/kernel/celltypes.h

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
    Json(#[from] serde_json::Error),
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

pub mod constids {
    pub mod parameter_names {
        pub const A_SIGNED: &str = "A_SIGNED";
        pub const A_WIDTH: &str = "A_WIDTH";
        pub const B_SIGNED: &str = "B_SIGNED";
        pub const B_WIDTH: &str = "B_WIDTH";
        pub const LUT: &str = "LUT";
        pub const S_WIDTH: &str = "S_WIDTH";
        pub const WIDTH: &str = "WIDTH";
        pub const Y_WIDTH: &str = "Y_WIDTH";
    }

    pub mod attribute_keys {
        pub const ALWAYS_COMB: &str = "always_comb";
        pub const ALWAYS_FF: &str = "always_ff";
        pub const ALWAYS_LATCH: &str = "always_latch";
        pub const DYNPORTS: &str = "dynports";
        pub const ENUM_TYPE: &str = "enum_type";
        pub const ENUM_VALUE_00: &str = "enum_value_00";
        pub const ENUM_VALUE_01: &str = "enum_value_01";
        pub const ENUM_VALUE_0: &str = "enum_value_0";
        pub const ENUM_VALUE_10: &str = "enum_value_10";
        pub const ENUM_VALUE_11: &str = "enum_value_11";
        pub const ENUM_VALUE_1: &str = "enum_value_1";
        pub const FULL_CASE: &str = "full_case";
        pub const HDLNAME: &str = "hdlname";
        pub const SRC: &str = "src";
        pub const TOP: &str = "top";
        pub const WIRETYPE: &str = "wiretype";
    }

    pub mod port_names {
        pub const A: &str = "A";
        pub const AD: &str = "AD";
        pub const ADDR: &str = "ADDR";
        pub const ALOAD: &str = "ALOAD";
        pub const ARGS: &str = "ARGS";
        pub const ARST: &str = "ARST";
        pub const B: &str = "B";
        pub const BI: &str = "BI";
        pub const C: &str = "C";
        pub const CI: &str = "CI";
        pub const CLK: &str = "CLK";
        pub const CLR: &str = "CLR";
        pub const CO: &str = "CO";
        pub const CTRL_IN: &str = "CTRL_IN";
        pub const CTRL_OUT: &str = "CTRL_OUT";
        pub const D: &str = "D";
        pub const DAT: &str = "DAT";
        pub const DATA: &str = "DATA";
        pub const DST: &str = "DST";
        pub const E: &str = "E";
        pub const EN: &str = "EN";
        pub const EN_DST: &str = "EN_DST";
        pub const EN_SRC: &str = "EN_SRC";
        pub const F: &str = "F";
        pub const G: &str = "G";
        pub const H: &str = "H";
        pub const I: &str = "I";
        pub const J: &str = "J";
        pub const K: &str = "K";
        pub const L: &str = "L";
        pub const M: &str = "M";
        pub const N: &str = "N";
        pub const O: &str = "O";
        pub const P: &str = "P";
        pub const Q: &str = "Q";
        pub const R: &str = "R";
        pub const RD_ADDR: &str = "RD_ADDR";
        pub const RD_ARST: &str = "RD_ARST";
        pub const RD_CLK: &str = "RD_CLK";
        pub const RD_DATA: &str = "RD_DATA";
        pub const RD_EN: &str = "RD_EN";
        pub const RD_SRST: &str = "RD_SRST";
        pub const S: &str = "S";
        pub const SET: &str = "SET";
        pub const SRC: &str = "SRC";
        pub const SRST: &str = "SRST";
        pub const T: &str = "T";
        pub const TRG: &str = "TRG";
        pub const U: &str = "U";
        pub const V: &str = "V";
        pub const WR_ADDR: &str = "WR_ADDR";
        pub const WR_CLK: &str = "WR_CLK";
        pub const WR_DATA: &str = "WR_DATA";
        pub const WR_EN: &str = "WR_EN";
        pub const X: &str = "X";
        pub const Y: &str = "Y";
    }

    pub mod internal_cells {
        pub const ADD: &str = "$add";
        pub const ADFF: &str = "$adff";
        pub const ADFFE: &str = "$adffe";
        pub const ADLATCH: &str = "$adlatch";
        pub const ALDFF: &str = "$aldff";
        pub const ALDFFE: &str = "$aldffe";
        pub const ALLCONST: &str = "$allconst";
        pub const ALLSEQ: &str = "$allseq";
        pub const ALU: &str = "$alu";
        pub const AND: &str = "$and";
        pub const ANYCONST: &str = "$anyconst";
        pub const ANYINIT: &str = "$anyinit";
        pub const ANYSEQ: &str = "$anyseq";
        pub const ASSERT: &str = "$assert";
        pub const ASSUME: &str = "$assume";
        pub const BMUX: &str = "$bmux";
        pub const BWEQX: &str = "$bweqx";
        pub const BWMUX: &str = "$bwmux";
        pub const CHECK: &str = "$check";
        pub const CONCAT: &str = "$concat";
        pub const COVER: &str = "$cover";
        pub const DEMUX: &str = "$demux";
        pub const DFF: &str = "$dff";
        pub const DFFE: &str = "$dffe";
        pub const DFFSR: &str = "$dffsr";
        pub const DFFSRE: &str = "$dffsre";
        pub const DIV: &str = "$div";
        pub const DIVFLOOR: &str = "$divfloor";
        pub const DLATCH: &str = "$dlatch";
        pub const DLATCHSR: &str = "$dlatchsr";
        pub const EQ: &str = "$eq";
        pub const EQUIV: &str = "$equiv";
        pub const EQX: &str = "$eqx";
        pub const FA: &str = "$fa";
        pub const FAIR: &str = "$fair";
        pub const FF: &str = "$ff";
        pub const FSM: &str = "$fsm";
        pub const FUTURE_FF: &str = "$future_ff";
        pub const GE: &str = "$ge";
        pub const GET_TAG: &str = "$get_tag";
        pub const GT: &str = "$gt";
        pub const INITSTATE: &str = "$initstate";
        pub const LCU: &str = "$lcu";
        pub const LE: &str = "$le";
        pub const LIVE: &str = "$live";
        pub const LOGIC_AND: &str = "$logic_and";
        pub const LOGIC_NOT: &str = "$logic_not";
        pub const LOGIC_OR: &str = "$logic_or";
        pub const LT: &str = "$lt";
        pub const LUT: &str = "$lut";
        pub const MACC: &str = "$macc";
        pub const MEM: &str = "$mem";
        pub const MEMINIT: &str = "$meminit";
        pub const MEMINIT_V2: &str = "$meminit_v2";
        pub const MEMRD: &str = "$memrd";
        pub const MEMRD_V2: &str = "$memrd_v2";
        pub const MEMWR: &str = "$memwr";
        pub const MEMWR_V2: &str = "$memwr_v2";
        pub const MEM_V2: &str = "$mem_v2";
        pub const MOD: &str = "$mod";
        pub const MODFLOOR: &str = "$modfloor";
        pub const MUL: &str = "$mul";
        pub const MUX: &str = "$mux";
        pub const NE: &str = "$ne";
        pub const NEG: &str = "$neg";
        pub const NEX: &str = "$nex";
        pub const NOT: &str = "$not";
        pub const OR: &str = "$or";
        pub const ORIGINAL_TAG: &str = "$original_tag";
        pub const OVERWRITE_TAG: &str = "$overwrite_tag";
        pub const PMUX: &str = "$pmux";
        pub const POS: &str = "$pos";
        pub const POW: &str = "$pow";
        pub const PRINT: &str = "$print";
        pub const REDUCE_AND: &str = "$reduce_and";
        pub const REDUCE_BOOL: &str = "$reduce_bool";
        pub const REDUCE_OR: &str = "$reduce_or";
        pub const REDUCE_XNOR: &str = "$reduce_xnor";
        pub const REDUCE_XOR: &str = "$reduce_xor";
        pub const SCOPEINFO: &str = "$scopeinfo";
        pub const SDFF: &str = "$sdff";
        pub const SDFFCE: &str = "$sdffce";
        pub const SDFFE: &str = "$sdffe";
        pub const SET_TAG: &str = "$set_tag";
        pub const SHIFT: &str = "$shift";
        pub const SHIFTX: &str = "$shiftx";
        pub const SHL: &str = "$shl";
        pub const SHR: &str = "$shr";
        pub const SLICE: &str = "$slice";
        pub const SOP: &str = "$sop";
        pub const SPECIFY2: &str = "$specify2";
        pub const SPECIFY3: &str = "$specify3";
        pub const SPECRULE: &str = "$specrule";
        pub const SR: &str = "$sr";
        pub const SSHL: &str = "$sshl";
        pub const SSHR: &str = "$sshr";
        pub const SUB: &str = "$sub";
        pub const TRIBUF: &str = "$tribuf";
        pub const XNOR: &str = "$xnor";
        pub const XOR: &str = "$xor";
    }

    pub mod std_cells {
        pub const ALDFFE_NNN: &str = "$_ALDFFE_NNN_";
        pub const ALDFFE_NNP: &str = "$_ALDFFE_NNP_";
        pub const ALDFFE_NPN: &str = "$_ALDFFE_NPN_";
        pub const ALDFFE_NPP: &str = "$_ALDFFE_NPP_";
        pub const ALDFFE_PNN: &str = "$_ALDFFE_PNN_";
        pub const ALDFFE_PNP: &str = "$_ALDFFE_PNP_";
        pub const ALDFFE_PPN: &str = "$_ALDFFE_PPN_";
        pub const ALDFFE_PPP: &str = "$_ALDFFE_PPP_";
        pub const ALDFF_NN: &str = "$_ALDFF_NN_";
        pub const ALDFF_NP: &str = "$_ALDFF_NP_";
        pub const ALDFF_PN: &str = "$_ALDFF_PN_";
        pub const ALDFF_PP: &str = "$_ALDFF_PP_";
        pub const AND: &str = "$_AND_";
        pub const ANDNOT: &str = "$_ANDNOT_";
        pub const AOI3: &str = "$_AOI3_";
        pub const AOI4: &str = "$_AOI4_";
        pub const BUF: &str = "$_BUF_";
        pub const DFFE_NN0N: &str = "$_DFFE_NN0N_";
        pub const DFFE_NN0P: &str = "$_DFFE_NN0P_";
        pub const DFFE_NN1N: &str = "$_DFFE_NN1N_";
        pub const DFFE_NN1P: &str = "$_DFFE_NN1P_";
        pub const DFFE_NN: &str = "$_DFFE_NN_";
        pub const DFFE_NP0N: &str = "$_DFFE_NP0N_";
        pub const DFFE_NP0P: &str = "$_DFFE_NP0P_";
        pub const DFFE_NP1N: &str = "$_DFFE_NP1N_";
        pub const DFFE_NP1P: &str = "$_DFFE_NP1P_";
        pub const DFFE_NP: &str = "$_DFFE_NP_";
        pub const DFFE_PN0N: &str = "$_DFFE_PN0N_";
        pub const DFFE_PN0P: &str = "$_DFFE_PN0P_";
        pub const DFFE_PN1N: &str = "$_DFFE_PN1N_";
        pub const DFFE_PN1P: &str = "$_DFFE_PN1P_";
        pub const DFFE_PN: &str = "$_DFFE_PN_";
        pub const DFFE_PP0N: &str = "$_DFFE_PP0N_";
        pub const DFFE_PP0P: &str = "$_DFFE_PP0P_";
        pub const DFFE_PP1N: &str = "$_DFFE_PP1N_";
        pub const DFFE_PP1P: &str = "$_DFFE_PP1P_";
        pub const DFFE_PP: &str = "$_DFFE_PP_";
        pub const DFFSRE_NNNN: &str = "$_DFFSRE_NNNN_";
        pub const DFFSRE_NNNP: &str = "$_DFFSRE_NNNP_";
        pub const DFFSRE_NNPN: &str = "$_DFFSRE_NNPN_";
        pub const DFFSRE_NNPP: &str = "$_DFFSRE_NNPP_";
        pub const DFFSRE_NPNN: &str = "$_DFFSRE_NPNN_";
        pub const DFFSRE_NPNP: &str = "$_DFFSRE_NPNP_";
        pub const DFFSRE_NPPN: &str = "$_DFFSRE_NPPN_";
        pub const DFFSRE_NPPP: &str = "$_DFFSRE_NPPP_";
        pub const DFFSRE_PNNN: &str = "$_DFFSRE_PNNN_";
        pub const DFFSRE_PNNP: &str = "$_DFFSRE_PNNP_";
        pub const DFFSRE_PNPN: &str = "$_DFFSRE_PNPN_";
        pub const DFFSRE_PNPP: &str = "$_DFFSRE_PNPP_";
        pub const DFFSRE_PPNN: &str = "$_DFFSRE_PPNN_";
        pub const DFFSRE_PPNP: &str = "$_DFFSRE_PPNP_";
        pub const DFFSRE_PPPN: &str = "$_DFFSRE_PPPN_";
        pub const DFFSRE_PPPP: &str = "$_DFFSRE_PPPP_";
        pub const DFFSR_NNN: &str = "$_DFFSR_NNN_";
        pub const DFFSR_NNP: &str = "$_DFFSR_NNP_";
        pub const DFFSR_NPN: &str = "$_DFFSR_NPN_";
        pub const DFFSR_NPP: &str = "$_DFFSR_NPP_";
        pub const DFFSR_PNN: &str = "$_DFFSR_PNN_";
        pub const DFFSR_PNP: &str = "$_DFFSR_PNP_";
        pub const DFFSR_PPN: &str = "$_DFFSR_PPN_";
        pub const DFFSR_PPP: &str = "$_DFFSR_PPP_";
        pub const DFF_N: &str = "$_DFF_N_";
        pub const DFF_NN0: &str = "$_DFF_NN0_";
        pub const DFF_NN1: &str = "$_DFF_NN1_";
        pub const DFF_NP0: &str = "$_DFF_NP0_";
        pub const DFF_NP1: &str = "$_DFF_NP1_";
        pub const DFF_P: &str = "$_DFF_P_";
        pub const DFF_PN0: &str = "$_DFF_PN0_";
        pub const DFF_PN1: &str = "$_DFF_PN1_";
        pub const DFF_PP0: &str = "$_DFF_PP0_";
        pub const DFF_PP1: &str = "$_DFF_PP1_";
        pub const DLATCHSR_NNN: &str = "$_DLATCHSR_NNN_";
        pub const DLATCHSR_NNP: &str = "$_DLATCHSR_NNP_";
        pub const DLATCHSR_NPN: &str = "$_DLATCHSR_NPN_";
        pub const DLATCHSR_NPP: &str = "$_DLATCHSR_NPP_";
        pub const DLATCHSR_PNN: &str = "$_DLATCHSR_PNN_";
        pub const DLATCHSR_PNP: &str = "$_DLATCHSR_PNP_";
        pub const DLATCHSR_PPN: &str = "$_DLATCHSR_PPN_";
        pub const DLATCHSR_PPP: &str = "$_DLATCHSR_PPP_";
        pub const DLATCH_N: &str = "$_DLATCH_N_";
        pub const DLATCH_NN0: &str = "$_DLATCH_NN0_";
        pub const DLATCH_NN1: &str = "$_DLATCH_NN1_";
        pub const DLATCH_NP0: &str = "$_DLATCH_NP0_";
        pub const DLATCH_NP1: &str = "$_DLATCH_NP1_";
        pub const DLATCH_P: &str = "$_DLATCH_P_";
        pub const DLATCH_PN0: &str = "$_DLATCH_PN0_";
        pub const DLATCH_PN1: &str = "$_DLATCH_PN1_";
        pub const DLATCH_PP0: &str = "$_DLATCH_PP0_";
        pub const DLATCH_PP1: &str = "$_DLATCH_PP1_";
        pub const FF: &str = "$_FF_";
        pub const MUX16: &str = "$_MUX16_";
        pub const MUX4: &str = "$_MUX4_";
        pub const MUX8: &str = "$_MUX8_";
        pub const MUX: &str = "$_MUX_";
        pub const NAND: &str = "$_NAND_";
        pub const NMUX: &str = "$_NMUX_";
        pub const NOR: &str = "$_NOR_";
        pub const NOT: &str = "$_NOT_";
        pub const OAI3: &str = "$_OAI3_";
        pub const OAI4: &str = "$_OAI4_";
        pub const OR: &str = "$_OR_";
        pub const ORNOT: &str = "$_ORNOT_";
        pub const SDFFCE_NN0N: &str = "$_SDFFCE_NN0N_";
        pub const SDFFCE_NN0P: &str = "$_SDFFCE_NN0P_";
        pub const SDFFCE_NN1N: &str = "$_SDFFCE_NN1N_";
        pub const SDFFCE_NN1P: &str = "$_SDFFCE_NN1P_";
        pub const SDFFCE_NP0N: &str = "$_SDFFCE_NP0N_";
        pub const SDFFCE_NP0P: &str = "$_SDFFCE_NP0P_";
        pub const SDFFCE_NP1N: &str = "$_SDFFCE_NP1N_";
        pub const SDFFCE_NP1P: &str = "$_SDFFCE_NP1P_";
        pub const SDFFCE_PN0N: &str = "$_SDFFCE_PN0N_";
        pub const SDFFCE_PN0P: &str = "$_SDFFCE_PN0P_";
        pub const SDFFCE_PN1N: &str = "$_SDFFCE_PN1N_";
        pub const SDFFCE_PN1P: &str = "$_SDFFCE_PN1P_";
        pub const SDFFCE_PP0N: &str = "$_SDFFCE_PP0N_";
        pub const SDFFCE_PP0P: &str = "$_SDFFCE_PP0P_";
        pub const SDFFCE_PP1N: &str = "$_SDFFCE_PP1N_";
        pub const SDFFCE_PP1P: &str = "$_SDFFCE_PP1P_";
        pub const SDFFE_NN0N: &str = "$_SDFFE_NN0N_";
        pub const SDFFE_NN0P: &str = "$_SDFFE_NN0P_";
        pub const SDFFE_NN1N: &str = "$_SDFFE_NN1N_";
        pub const SDFFE_NN1P: &str = "$_SDFFE_NN1P_";
        pub const SDFFE_NP0N: &str = "$_SDFFE_NP0N_";
        pub const SDFFE_NP0P: &str = "$_SDFFE_NP0P_";
        pub const SDFFE_NP1N: &str = "$_SDFFE_NP1N_";
        pub const SDFFE_NP1P: &str = "$_SDFFE_NP1P_";
        pub const SDFFE_PN0N: &str = "$_SDFFE_PN0N_";
        pub const SDFFE_PN0P: &str = "$_SDFFE_PN0P_";
        pub const SDFFE_PN1N: &str = "$_SDFFE_PN1N_";
        pub const SDFFE_PN1P: &str = "$_SDFFE_PN1P_";
        pub const SDFFE_PP0N: &str = "$_SDFFE_PP0N_";
        pub const SDFFE_PP0P: &str = "$_SDFFE_PP0P_";
        pub const SDFFE_PP1N: &str = "$_SDFFE_PP1N_";
        pub const SDFFE_PP1P: &str = "$_SDFFE_PP1P_";
        pub const SDFF_NN0: &str = "$_SDFF_NN0_";
        pub const SDFF_NN1: &str = "$_SDFF_NN1_";
        pub const SDFF_NP0: &str = "$_SDFF_NP0_";
        pub const SDFF_NP1: &str = "$_SDFF_NP1_";
        pub const SDFF_PN0: &str = "$_SDFF_PN0_";
        pub const SDFF_PN1: &str = "$_SDFF_PN1_";
        pub const SDFF_PP0: &str = "$_SDFF_PP0_";
        pub const SDFF_PP1: &str = "$_SDFF_PP1_";
        pub const SR_NN: &str = "$_SR_NN_";
        pub const SR_NP: &str = "$_SR_NP_";
        pub const SR_PN: &str = "$_SR_PN_";
        pub const SR_PP: &str = "$_SR_PP_";
        pub const TBUF: &str = "$_TBUF_";
        pub const XNOR: &str = "$_XNOR_";
        pub const XOR: &str = "$_XOR_";
    }
}

#[cfg(test)]
mod tests {
    use super::constids::*;
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
                        parameter_names::A_SIGNED.to_string(),
                        "00000000000000000000000000000001".to_string()
                    ),
                    (
                        parameter_names::A_WIDTH.to_string(),
                        "00000000000000000000000000000100".to_string()
                    )
                ]),
                attributes: FnvHashMap::from_iter([(
                    attribute_keys::SRC.to_string(),
                    "test.v".to_string()
                )]),
                port_directions: FnvHashMap::from_iter([(
                    port_names::A.to_string(),
                    PortDirection::Output
                )]),
                connections: FnvHashMap::from_iter([(
                    port_names::A.to_string(),
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
                attributes: FnvHashMap::from_iter([(
                    attribute_keys::SRC.to_string(),
                    "test.v".to_string()
                )]),
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
                attributes: FnvHashMap::from_iter([(
                    attribute_keys::SRC.to_string(),
                    "test.v".to_string()
                )]),
                offset: 0,
                upto: 0,
                signed: 0
            }
        )
    }

    #[test]
    fn test_parse_lut_cell() {
        let json = r#"{
    "hide_name": 1,
    "type": "$lut",
    "parameters": {
        "LUT": "1010110000000000",
        "WIDTH": "00000000000000000000000000000100"
    },
    "attributes": {
    },
    "port_directions": {
        "A": "input",
        "Y": "output"
    },
    "connections": {
        "A": [ 8, 6, 9, 3 ],
        "Y": [ 10 ]
    }
}"#;
        let parsed: Cell = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            Cell {
                hide_name: 1,
                ty: internal_cells::LUT.to_string(),
                parameters: FnvHashMap::from_iter([
                    (
                        parameter_names::LUT.to_string(),
                        "1010110000000000".to_string()
                    ),
                    (
                        parameter_names::WIDTH.to_string(),
                        "00000000000000000000000000000100".to_string()
                    )
                ]),
                attributes: FnvHashMap::default(),
                port_directions: FnvHashMap::from_iter([
                    (port_names::A.to_string(), PortDirection::Input),
                    (port_names::Y.to_string(), PortDirection::Output)
                ]),
                connections: FnvHashMap::from_iter([
                    (
                        port_names::A.to_string(),
                        vec![
                            SignalBit::Ref(8),
                            SignalBit::Ref(6),
                            SignalBit::Ref(9),
                            SignalBit::Ref(3)
                        ]
                    ),
                    (port_names::Y.to_string(), vec![SignalBit::Ref(10)])
                ])
            }
        );
    }
}
