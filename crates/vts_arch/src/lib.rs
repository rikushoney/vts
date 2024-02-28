use std::sync::Arc;

use fnv::FnvHashMap as HashMap;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Module {
    pub name: Arc<str>,
    pub cells: HashMap<Arc<str>, Arc<Component>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Component {
    pub ports: HashMap<Arc<str>, Arc<Port>>,
    pub children: HashMap<Arc<str>, Arc<Component>>,
    pub class: Option<ComponentClass>,
}

impl Component {
    pub fn add_port(&mut self, name: &str, port: Port) -> Result<Arc<Port>, ()> {
        let port = Arc::new(port);
        self.ports.insert(name.into(), port.clone());
        Ok(port)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
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

impl Port {
    pub fn new(kind: PortKind, n_pins: usize, class: Option<PortClass>) -> Self {
        Self {
            kind,
            n_pins,
            class,
        }
    }
}
