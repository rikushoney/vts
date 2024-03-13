use std::collections::HashMap;

use serde::Deserialize;

use crate::arch::{
    assert_ptr_eq, ComponentId, Module, Port, PortClass, PortId, PortKind, StringId,
};

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

    pub fn add_port(
        &'m mut self,
        module: &'m mut Module<'m>,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> &Port<'m> {
        assert_ptr_eq!(module, self.module);

        let name = module.strings.entry(name);
        let port = Port::new(self, name, kind, n_pins, class);
        let id = module.ports.entry(port);
        match module.port_name_map.insert(name, id) {
            Some(_) => {
                let name = module.strings.lookup(name);
                panic!(r#""{name}" already in module"#);
            }
            None => module.ports.lookup(id),
        }
    }
}
