pub mod de;
pub mod ser;

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::arch::{component::ComponentData, impl_dbkey_wrapper, ComponentId, Module, StringId};

impl_dbkey_wrapper!(PortId, u32);

impl PortId {
    pub fn to_port(self, module: &Module) -> Port<'_> {
        Port::new(module, self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct PortData {
    name: StringId,
    parent: ComponentId,
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}

impl PortData {
    fn new(
        module: &mut Module,
        parent: &mut ComponentData,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> Self {
        let name = module.strings.entry(name);
        assert!(
            parent.ports.get(&name).is_none(),
            r#"port "{port}" already in component "{component}""#,
            port = module.strings.lookup(name),
            component = module.strings.lookup(parent.name),
        );

        let parent = *module
            .components
            .get(&parent.name)
            .expect("parent component not in module");

        Self {
            name,
            parent,
            kind,
            n_pins,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.lookup(self.name)
    }

    pub fn rename<'m>(&'m mut self, module: &'m mut Module, name: &str) {
        let name = module.strings.entry(name);
        let parent = module.component_mut(self.parent);
        assert!(
            parent.ports.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = module.strings.lookup(name),
            module = module.strings.lookup(module.name),
        );

        let port = parent
            .ports
            .remove(&self.name)
            .expect("port should be in module");
        parent.ports.insert(name, port);
        self.name = name;
    }

    pub fn parent(&self) -> ComponentId {
        self.parent
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port<'m> {
    module: &'m Module,
    id: PortId,
    data: &'m PortData,
}

impl<'m> Port<'m> {
    fn new(module: &'m Module, id: PortId) -> Self {
        let data = module.port_db.lookup(id);

        Self { module, id, data }
    }

    pub fn select(&self, range: Range<u32>) -> PinRange {
        PinRange::new(self.id, range)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinRange {
    port: PortId,
    range: Range<u32>,
}

impl PinRange {
    fn new(port: PortId, range: Range<u32>) -> Self {
        Self { port, range }
    }
}
