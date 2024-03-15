use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use super::{impl_dbkey_wrapper, Component, Module, StringId};

impl_dbkey_wrapper!(PortId, u32);

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
pub struct Port {
    name: StringId,
    pub kind: PortKind,
    pub n_pins: usize,
    pub class: Option<PortClass>,
}

impl Port {
    pub(crate) fn new(
        module: &mut Module,
        parent: &mut Component,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> Port {
        let name = module.strings.entry(name);
        if parent.ports.get(&name).is_some() {
            let name = module.strings.lookup(name);
            let component_name = module.strings.lookup(parent.name);
            panic!(r#"port "{name}" already in component "{component_name}""#)
        }

        Self {
            name,
            kind,
            n_pins,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.lookup(self.name)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PortRecipe {
    pub(crate) name: Option<String>,
    kind: Option<PortKind>,
    n_pins: Option<usize>,
    class: Option<PortClass>,
}

impl PortRecipe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn kind(&mut self, kind: PortKind) -> &mut Self {
        self.kind = Some(kind);
        self
    }

    pub fn n_pins(&mut self, n_pins: usize) -> &mut Self {
        self.n_pins = Some(n_pins);
        self
    }

    pub fn class(&mut self, class: PortClass) -> &mut Self {
        self.class = Some(class);
        self
    }

    pub fn instantiate(&self, module: &mut Module, component: &mut Component) -> PortId {
        let port = Port::new(
            module,
            component,
            self.name.as_ref().expect("port must have a name").as_str(),
            self.kind.expect("port must have a kind"),
            self.n_pins.unwrap_or(1),
            self.class,
        );

        let name = port.name;
        let port = module.ports.entry(port);

        if component.ports.insert(name, port).is_some() {
            let port_name = module.strings.lookup(name);
            let module_name = module.strings.lookup(name);
            panic!(r#"port "{port_name}" already in module "{module_name}""#)
        }

        port
    }
}

pub(crate) struct PortSerializer<'m> {
    pub(crate) module: &'m Module,
    pub(crate) port: &'m Port,
}

impl<'m> Serialize for PortSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut port_serializer = serializer.serialize_struct("Port", 4)?;

        let name = self.module.strings.lookup(self.port.name);
        port_serializer.serialize_field("name", name)?;

        port_serializer.serialize_field("kind", &self.port.kind)?;
        port_serializer.serialize_field("n_pins", &self.port.n_pins)?;
        port_serializer.serialize_field("class", &self.port.class)?;

        port_serializer.end()
    }
}
