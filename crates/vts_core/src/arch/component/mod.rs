pub mod de;
pub mod ser;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::arch::{impl_dbkey_wrapper, port::PortData, Module, Port, StringId};

impl_dbkey_wrapper!(Component, u32);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentData {
    pub(crate) name: StringId,
    pub(crate) ports: HashMap<StringId, Port>,
    references: HashMap<StringId, Component>,
    pub class: Option<ComponentClass>,
}

impl ComponentData {
    fn new(module: &mut Module, name: &str, class: Option<ComponentClass>) -> Self {
        let name = module.strings.entry(name);
        assert!(
            module.components.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = module.strings.lookup(name),
            module = module.strings.lookup(module.name)
        );

        let ports = HashMap::default();
        let references = HashMap::default();

        Self {
            name,
            ports,
            references,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.lookup(self.name)
    }

    pub fn set_name<'m>(&'m mut self, module: &'m mut Module, name: &str) {
        let name = module.strings.entry(name);
        assert!(
            module.components.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = module.strings.lookup(name),
            module = module.strings.lookup(module.name)
        );

        let component = module
            .components
            .remove(&self.name)
            .expect("component should be in module");

        module.components.insert(name, component);
        self.name = name;
    }

    pub fn port<'m>(&self, module: &'m Module, port: Port) -> &'m PortData {
        assert!(
            self.ports.values().any(|p| p == &port),
            r#"port "{port}" not in component "{component}""#,
            port = module.port_db.lookup(port).name(module),
            component = self.name(module),
        );
        module.get_data(port)
    }

    pub fn port_mut<'m>(&'m self, module: &'m mut Module, port: Port) -> &'m mut PortData {
        assert!(
            self.ports.values().any(|p| p == &port),
            r#"port "{port}" not in component "{component}""#,
            port = module.port_db.lookup(port).name(module),
            component = self.name(module),
        );
        module.get_data_mut(port)
    }

    pub fn port_id(&self, module: &Module, name: &str) -> Option<Port> {
        let name = module.strings.rlookup(name)?;
        self.ports.get(&name).copied()
    }
}
