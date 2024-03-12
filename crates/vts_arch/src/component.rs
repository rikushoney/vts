use std::collections::HashMap;

use serde::Deserialize;

use crate::{ComponentId, Module, Port, PortId, StringId};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component<'m> {
    pub(crate) module: &'m Module<'m>,
    pub(crate) name: StringId,
    ports: HashMap<StringId, PortId>,
    references: HashMap<StringId, ComponentId>,
    pub class: Option<ComponentClass>,
}

impl<'m> Component<'m> {
    pub fn new(module: &'m mut Module, name: &str, class: Option<ComponentClass>) -> Self {
        let name = module.strings.entry(name);
        let ports = HashMap::default();
        let references = HashMap::default();

        Self {
            module,
            name,
            ports,
            references,
            class,
        }
    }

    pub fn name(&self) -> &str {
        self.module.strings.lookup(self.name)
    }

    pub fn port(&self, name: &str) -> Option<&Port<'m>> {
        let name = self.module.strings.rlookup(name)?;
        let id = *self.ports.get(&name)?;

        Some(self.module.ports.lookup(id))
    }
}
