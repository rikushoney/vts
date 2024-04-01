pub mod de;
pub mod ser;

use std::collections::{hash_map, HashMap, HashSet};
use std::ops::Index;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::arch::{
    impl_dbkey_wrapper,
    port::{PinRange, Port, PortBuilder, PortData},
    Module, PortId, StringId,
};

impl_dbkey_wrapper!(ComponentId, u32);

impl ComponentId {
    pub fn to_component(self, module: &Module) -> Component<'_> {
        Component::new(module, self)
    }
}

impl_dbkey_wrapper!(ComponentRefId, u32);

impl ComponentRefId {
    pub fn to_component(self, module: &Module) -> Component<'_> {
        let reference = module[self];
        Component::new(module, reference.component)
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
    pub(crate) references: HashMap<StringId, ComponentRefId>,
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

        if let Some(component) = module.components.remove(&self.name) {
            module.components.insert(name, component);
        }

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
pub struct ComponentRefData {
    component: ComponentId,
    pub n_instances: usize,
}

impl ComponentRefData {
    pub(crate) fn new(component: ComponentId, n_instances: usize) -> Self {
        Self {
            component,
            n_instances,
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

    pub fn name(&self) -> &str {
        self.data.name(self.module)
    }

    pub fn ports(&self) -> PortIter {
        PortIter {
            module: self.module,
            iter: self.data.ports.values(),
        }
    }

    pub fn references(&self) -> ComponentRefIter {
        ComponentRefIter {
            module: self.module,
            iter: self.data.references.iter(),
        }
    }

    pub fn class(&self) -> Option<ComponentClass> {
        self.data.class
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRef<'m> {
    module: &'m Module,
    id: ComponentRefId,
    data: &'m ComponentRefData,
}

impl<'m> ComponentRef<'m> {
    fn new(module: &'m Module, id: ComponentRefId) -> Self {
        let data = &module[id];

        Self { module, id, data }
    }

    pub fn component(&self) -> Component<'m> {
        Component::new(self.module, self.data.component)
    }
}

impl<'m> PartialEq<ComponentRef<'m>> for &Component<'m> {
    fn eq(&self, other: &ComponentRef) -> bool {
        self.id == other.data.component
    }
}

impl<'m> Index<PortId> for Component<'m> {
    type Output = PortData;

    fn index(&self, port: PortId) -> &Self::Output {
        &self.module[port]
    }
}

pub struct PortIter<'m> {
    module: &'m Module,
    iter: hash_map::Values<'m, StringId, PortId>,
}

impl<'m> Iterator for PortIter<'m> {
    type Item = Port<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        let port = *self.iter.next()?;
        Some(port.to_port(self.module))
    }
}

pub struct ComponentRefIter<'m> {
    module: &'m Module,
    iter: hash_map::Iter<'m, StringId, ComponentRefId>,
}

impl<'m> Iterator for ComponentRefIter<'m> {
    type Item = (&'m str, ComponentRef<'m>);

    fn next(&mut self) -> Option<Self::Item> {
        let (&alias, &reference) = self.iter.next()?;
        let alias = &self.module.strings[alias];
        let reference = ComponentRef::new(self.module, reference);
        Some((alias, reference))
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
    unresolved_references: HashSet<ComponentWeakRef>,
    unresolved_named_references: HashMap<StringId, ComponentWeakRef>,
    name_is_set: bool,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ComponentWeakRef {
    pub(crate) component: StringId,
    pub(crate) n_instances: usize,
}

#[derive(Debug, Error)]
pub enum ComponentBuildError {
    #[error(r#"component "{component}" already in "{module}""#)]
    DuplicateComponent { module: String, component: String },
    #[error(r#"component "{reference}" already referenced in "{component}""#)]
    DuplicateReference {
        component: String,
        reference: String,
    },
    #[error("component must have a {0}")]
    MissingField(&'static str),
}

pub type ComponentBuildResult = Result<
    (
        ComponentId,
        HashSet<ComponentWeakRef>,
        HashMap<StringId, ComponentWeakRef>,
    ),
    ComponentBuildError,
>;

pub trait GetStringId {
    fn get_string_id(&self, module: &mut Module) -> StringId;
}

impl GetStringId for StringId {
    fn get_string_id(&self, _module: &mut Module) -> StringId {
        *self
    }
}

impl<S: AsRef<str>> GetStringId for S {
    fn get_string_id(&self, module: &mut Module) -> StringId {
        module.strings.entry(self.as_ref())
    }
}

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
        n_instances: Option<usize>,
    ) -> Result<ComponentRefId, ComponentBuildError> {
        let alias = match alias {
            Some(alias) => self.module.strings.entry(alias),
            None => self.module[component].name,
        };
        let n_instances = n_instances.unwrap_or(1);

        let reference = ComponentRefData::new(component, n_instances);
        let reference = self.module.reference_db.entry(reference);
        if self.data.references.insert(alias, reference).is_some() {
            let component = self.module.strings[self.module[component].name].to_string();
            let reference = self.module.strings[alias].to_string();
            Err(ComponentBuildError::DuplicateReference {
                component,
                reference,
            })
        } else {
            Ok(reference)
        }
    }

    pub fn add_weak_reference<S: GetStringId>(
        &mut self,
        component: S,
        alias: Option<S>,
        n_instances: Option<usize>,
    ) -> Result<ComponentWeakRef, ComponentBuildError> {
        let component = component.get_string_id(self.module);
        let reference = ComponentWeakRef {
            component,
            n_instances: n_instances.unwrap_or(1),
        };

        if let Some(alias) = alias {
            let alias = alias.get_string_id(self.module);
            if self
                .unresolved_named_references
                .insert(alias, reference)
                .is_some()
            {
                let parent = self.module.strings[self.data.name].to_string();
                let reference = self.module.strings[alias].to_string();
                return Err(ComponentBuildError::DuplicateReference {
                    component: parent,
                    reference,
                });
            }
        } else if !self.unresolved_references.insert(reference) {
            let parent = self.module.strings[self.data.name].to_string();
            let reference = self.module.strings[component].to_string();
            return Err(ComponentBuildError::DuplicateReference {
                component: parent,
                reference,
            });
        }

        Ok(reference)
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
            let module = self.module.strings[self.module.name].to_string();
            return Err(ComponentBuildError::DuplicateComponent { module, component });
        }

        #[cfg(debug_assertions)]
        {
            let component = self
                .module
                .strings
                .rlookup(self.module[component].name(self.module))
                .expect("component name should be in module strings");
            debug_assert_eq!(component, name);
            debug_assert!(self.module.components.contains_key(&component));
        }

        Ok((
            component,
            self.unresolved_references,
            self.unresolved_named_references,
        ))
    }
}
