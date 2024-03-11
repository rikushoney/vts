use std::sync::Arc;

use fnv::FnvHashMap as HashMap;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Module {
    pub name: Arc<str>,
    pub components: HashMap<Arc<str>, Arc<Component>>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let name = name.into();
        let components = HashMap::default();
        Self { name, components }
    }

    pub fn add_component(&mut self, component: Component) -> Arc<Component> {
        let component = Arc::new(component);
        self.components
            .insert(Arc::clone(&component.name), Arc::clone(&component));
        component
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Component {
    pub name: Arc<str>,
    pub ports: HashMap<Arc<str>, Arc<Port>>,
    pub children: HashMap<Arc<str>, Arc<Component>>,
    pub class: Option<ComponentClass>,
}

impl Component {
    pub fn new(name: &str, class: Option<ComponentClass>) -> Self {
        let name = name.into();
        let ports = HashMap::default();
        let children = HashMap::default();
        Self {
            name,
            ports,
            children,
            class,
        }
    }

    pub fn add_port(&mut self, port: Port) -> Arc<Port> {
        let port = Arc::new(port);
        self.ports.insert(Arc::clone(&port.name), Arc::clone(&port));
        port
    }
}

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

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Port {
    pub name: Arc<str>,
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}

impl Port {
    pub fn new(name: &str, kind: PortKind, n_pins: usize, class: Option<PortClass>) -> Self {
        let name = name.into();
        Self {
            name,
            kind,
            n_pins,
            class,
        }
    }
}
