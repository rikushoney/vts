pub mod de;
pub mod ser;

use std::collections::HashMap;
use std::ops::Index;

use serde::{Deserialize, Serialize};

use crate::arch::{
    impl_dbkey_wrapper,
    port::{PinRange, PortData},
    Module, PortId, StringId,
};

impl_dbkey_wrapper!(ComponentId, u32);

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRef(ComponentId);

impl ComponentId {
    pub fn reference(self) -> ComponentRef {
        ComponentRef(self)
    }

    pub fn to_component(self, module: &Module) -> Component<'_> {
        Component::new(module, self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentData {
    pub(crate) name: StringId,
    pub(crate) ports: HashMap<StringId, PortId>,
    pub(crate) references: HashMap<StringId, ComponentRef>,
    connections: Vec<Connection>,
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
        let connections = Vec::new();

        Self {
            name,
            ports,
            references,
            connections,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.lookup(self.name)
    }

    pub fn rename<'m>(&'m mut self, module: &'m mut Module, name: &str) {
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

    pub fn port<'m>(&self, module: &'m Module, port: PortId) -> &'m PortData {
        assert!(
            self.ports.values().any(|p| p == &port),
            r#"port "{port}" not in component "{component}""#,
            port = module.port_db.lookup(port).name(module),
            component = self.name(module),
        );
        module.get_data(port)
    }

    pub fn port_mut<'m>(&'m self, module: &'m mut Module, port: PortId) -> &'m mut PortData {
        assert!(
            self.ports.values().any(|p| p == &port),
            r#"port "{port}" not in component "{component}""#,
            port = module.port_db.lookup(port).name(module),
            component = self.name(module),
        );
        module.get_data_mut(port)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component<'m> {
    module: &'m Module,
    id: ComponentId,
    data: &'m ComponentData,
}

impl<'m> Component<'m> {
    fn new(module: &'m Module, id: ComponentId) -> Self {
        let data = module.component_db.lookup(id);

        Self { module, id, data }
    }
}

impl<'m> Index<PortId> for Component<'m> {
    type Output = PortData;

    fn index(&self, port: PortId) -> &Self::Output {
        self.data.port(self.module, port)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Connection {
    source: PinRange,
    sink: PinRange,
}

impl Connection {
    pub fn new(source: PinRange, sink: PinRange) -> Self {
        Self { source, sink }
    }
}
