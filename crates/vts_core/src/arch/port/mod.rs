pub mod de;
pub mod ser;

use std::ops::Range;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::arch::{ComponentId, Module, StringId};

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
        parent: ComponentId,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> Self {
        let name = module.strings.entry(name);
        let component = &module[parent];
        assert!(
            component.ports.get(&name).is_none(),
            r#"port "{port}" already in component "{component}""#,
            port = &module.strings[name],
            component = &module.strings[component.name],
        );

        Self {
            name,
            parent,
            kind,
            n_pins,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        &module.strings[self.name]
    }

    pub fn rename<'m>(&'m mut self, module: &'m mut Module, name: &str) {
        let name = module.strings.entry(name);
        let parent = &mut module[self.parent];
        assert!(
            parent.ports.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = &module.strings[name],
            module = &module.strings[module.name],
        );

        if let Some(port) = parent.ports.remove(&self.name) {
            parent.ports.insert(name, port);
        }

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
        let data = &module.port_db[id];

        Self { module, id, data }
    }

    pub fn name(&self) -> &str {
        &self.module.strings[self.data.name]
    }

    pub fn kind(&self) -> PortKind {
        self.data.kind
    }

    pub fn n_pins(&self) -> usize {
        self.data.n_pins
    }

    pub fn class(&self) -> Option<PortClass> {
        self.data.class
    }

    pub fn select(&self, range: Range<u32>) -> PortPins {
        PortPins::new(self.id, range)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PortPins {
    port: PortId,
    range: Range<u32>,
}

impl PortPins {
    fn new(port: PortId, range: Range<u32>) -> Self {
        Self { port, range }
    }

    pub fn start(&self) -> u32 {
        self.range.start
    }

    pub fn end(&self) -> u32 {
        self.range.end
    }

    pub fn range(&self) -> Range<u32> {
        self.range.clone()
    }

    pub fn port<'m>(&self, module: &'m Module) -> Port<'m> {
        Port::new(module, self.port)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WeakPortPins {
    pub(crate) port: StringId,
    pub(crate) range: Range<u32>,
}

impl WeakPortPins {
    pub(crate) fn new(port: StringId, range: Range<u32>) -> Self {
        Self { port, range }
    }
}

pub struct PortBuilder<'m> {
    module: &'m mut Module,
    parent: ComponentId,
    data: PortData,
    name_is_set: bool,
    kind_is_set: bool,
    n_pins_is_set: bool,
}

#[derive(Debug, Error)]
pub enum PortBuildError {
    #[error(r#"port "{port}" already in "{module}""#)]
    DuplicatePort { module: String, port: String },
    #[error("port must have a {0}")]
    MissingField(&'static str),
}

impl<'m> PortBuilder<'m> {
    pub fn new(module: &'m mut Module, parent: ComponentId) -> Self {
        let data = PortData::new(module, parent, "", PortKind::Input, 1, None);

        Self {
            module,
            parent,
            data,
            name_is_set: false,
            kind_is_set: false,
            n_pins_is_set: false,
        }
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.data.rename(self.module, name);
        self.name_is_set = true;
        self
    }

    pub fn set_kind(&mut self, kind: PortKind) -> &mut Self {
        self.data.kind = kind;
        self.kind_is_set = true;
        self
    }

    pub fn set_n_pins(&mut self, n_pins: usize) -> &mut Self {
        self.data.n_pins = n_pins;
        self.n_pins_is_set = true;
        self
    }

    pub fn set_class(&mut self, class: PortClass) -> &mut Self {
        self.data.class = Some(class);
        self
    }

    pub fn is_name_set(&self) -> bool {
        self.name_is_set
    }

    pub fn is_kind_set(&self) -> bool {
        self.kind_is_set
    }

    pub fn is_n_pins_set(&self) -> bool {
        self.n_pins_is_set
    }

    pub fn is_class_set(&self) -> bool {
        self.data.class.is_some()
    }

    pub fn finish(self) -> Result<PortId, PortBuildError> {
        if !self.is_name_set() {
            return Err(PortBuildError::MissingField("name"));
        }

        if !self.is_kind_set() {
            return Err(PortBuildError::MissingField("kind"));
        }

        let name = self.data.name;
        let port = self.module.port_db.entry(self.data);

        if self.module[self.parent].ports.insert(name, port).is_some() {
            let port = self.module.strings[name].to_string();
            let module = self.module.strings[self.module.name].to_string();

            return Err(PortBuildError::DuplicatePort { module, port });
        }

        debug_assert!({
            let name = self
                .module
                .strings
                .rlookup(self.module[port].name(self.module))
                .expect("port name should be in module strings");
            self.module[self.parent].ports.contains_key(&name)
        });

        Ok(port)
    }
}
