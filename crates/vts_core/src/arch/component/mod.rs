pub mod de;
pub mod ser;

use std::collections::{HashMap, HashSet};
use std::ops::Index;

use serde::{Deserialize, Serialize};

use crate::arch::{
    impl_dbkey_wrapper,
    port::{PinRange, PortBuilder, PortData},
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
            component = &module.strings[name],
            module = &module.strings[module.name]
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
        &module.strings[self.name]
    }

    pub fn rename<'m>(&'m mut self, module: &'m mut Module, name: &str) {
        let name = module.strings.entry(name);
        assert!(
            module.components.get(&name).is_none(),
            r#"component "{component}" already in module "{module}""#,
            component = &module.strings[name],
            module = &module.strings[module.name]
        );

        let component = module
            .components
            .remove(&self.name)
            .expect("component should be in module");

        module.components.insert(name, component);
        self.name = name;
    }

    pub fn get_port<'m>(&self, module: &'m Module, port: PortId) -> Option<&'m PortData> {
        if self.ports.values().any(|p| p == &port) {
            Some(&module[port])
        } else {
            None
        }
    }

    pub fn get_port_mut<'m>(
        &'m self,
        module: &'m mut Module,
        port: PortId,
    ) -> Option<&'m mut PortData> {
        if self.ports.values().any(|p| p == &port) {
            Some(&mut module[port])
        } else {
            None
        }
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
        let data = &module.component_db[id];

        Self { module, id, data }
    }
}

impl<'m> Index<PortId> for Component<'m> {
    type Output = PortData;

    fn index(&self, port: PortId) -> &Self::Output {
        &self.module[port]
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

pub struct ComponentBuilder<'m> {
    pub(crate) module: &'m mut Module,
    pub(crate) data: ComponentData,
    unresolved_references: HashSet<StringId>,
    unresolved_named_references: HashMap<StringId, StringId>,
    name_is_set: bool,
}

pub enum ComponentBuildError {
    DuplicateComponent {
        module: String,
        component: String,
    },
    DuplicateReference {
        component: String,
        reference: String,
    },
    MissingField(&'static str),
}

pub type ComponentBuildResult =
    Result<(ComponentId, HashSet<StringId>, HashMap<StringId, StringId>), ComponentBuildError>;

impl<'m> ComponentBuilder<'m> {
    pub fn new(module: &'m mut Module) -> Self {
        let data = ComponentData::new(module, "", None);
        let unresolved_references = HashSet::new();
        let unresolved_named_references = HashMap::new();

        Self {
            module,
            data,
            unresolved_references,
            unresolved_named_references,
            name_is_set: false,
        }
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.data.rename(self.module, name);
        self.name_is_set = true;
        self
    }

    pub fn add_port(&mut self) -> PortBuilder<'_> {
        PortBuilder::new(self.module, &mut self.data)
    }

    pub fn add_reference(
        &mut self,
        component: ComponentId,
        alias: Option<&str>,
    ) -> Result<&mut Self, ComponentBuildError> {
        let alias = match alias {
            Some(alias) => self.module.strings.entry(alias),
            None => self.module[component].name,
        };

        let reference = component.reference();
        if self.data.references.insert(alias, reference).is_some() {
            let component = self.module.strings[self.module[component].name].to_string();
            let reference = self.module.strings[alias].to_string();
            Err(ComponentBuildError::DuplicateReference {
                component,
                reference,
            })
        } else {
            Ok(self)
        }
    }

    pub fn add_weak_reference(
        &mut self,
        component: StringId,
        alias: Option<StringId>,
    ) -> Result<&mut Self, ComponentBuildError> {
        if let Some(alias) = alias {
            if self
                .unresolved_named_references
                .insert(alias, component)
                .is_some()
            {
                let parent = self.module.strings[self.data.name].to_string();
                let reference = self.module.strings[alias].to_string();
                return Err(ComponentBuildError::DuplicateReference {
                    component: parent,
                    reference,
                });
            }
        } else if !self.unresolved_references.insert(component) {
            let parent = self.module.strings[self.data.name].to_string();
            let reference = self.module.strings[component].to_string();
            return Err(ComponentBuildError::DuplicateReference {
                component: parent,
                reference,
            });
        }

        Ok(self)
    }

    pub fn set_class(&mut self, class: ComponentClass) -> &mut Self {
        self.data.class = Some(class);
        self
    }

    pub fn is_name_set(&self) -> bool {
        self.name_is_set
    }

    pub fn is_ports_empty(&self) -> bool {
        self.data.ports.is_empty()
    }

    pub fn is_references_empty(&self) -> bool {
        self.data.references.is_empty()
    }

    pub fn is_unresolved_references_empty(&self) -> bool {
        self.unresolved_references.is_empty()
    }

    pub fn is_unresolved_named_references_empty(&self) -> bool {
        self.unresolved_named_references.is_empty()
    }

    pub fn is_class_set(&self) -> bool {
        self.data.class.is_some()
    }

    pub fn finish(self) -> ComponentBuildResult {
        if !self.is_name_set() {
            return Err(ComponentBuildError::MissingField("name"));
        }

        let name = self.data.name;
        let component = self.module.component_db.entry(self.data);

        if self.module.components.insert(name, component).is_some() {
            let component = self.module.strings[name].to_string();
            let module = self.module.strings[name].to_string();
            return Err(ComponentBuildError::DuplicateComponent { module, component });
        }

        debug_assert!({
            let name = self
                .module
                .strings
                .rlookup(self.module[component].name(self.module))
                .expect("component name should be in module strings");
            self.module.components.contains_key(&name)
        });

        Ok((
            component,
            self.unresolved_references,
            self.unresolved_named_references,
        ))
    }
}
