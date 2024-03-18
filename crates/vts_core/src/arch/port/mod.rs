pub mod de;
pub mod ser;

use serde::{Deserialize, Serialize};

use crate::arch::{component::ComponentData, impl_dbkey_wrapper, Component, Module, StringId};

impl_dbkey_wrapper!(Port, u32);

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
    parent: Component,
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

    pub fn set_name<'m>(&'m mut self, module: &'m mut Module, name: &str) {
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

    pub fn parent(&self) -> Component {
        self.parent
    }
}
