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
            port = &module.strings[name],
            component = &module.strings[parent.name],
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

pub struct PortBuilder<'m> {
    module: &'m mut Module,
    parent: &'m mut ComponentData,
    data: PortData,
    name_is_set: bool,
    kind_is_set: bool,
    n_pins_is_set: bool,
}

pub enum PortBuildError {
    DuplicatePort { module: String, port: String },
    MissingField(&'static str),
}

impl<'m> PortBuilder<'m> {
    pub fn new(module: &'m mut Module, parent: &'m mut ComponentData) -> Self {
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

        if self.parent.ports.insert(name, port).is_some() {
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
            self.parent.ports.contains_key(&name)
        });

        Ok(port)
    }
}
