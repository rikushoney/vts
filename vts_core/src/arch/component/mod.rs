#![allow(unused)] // TODO: remove this!

// pub mod de;
// pub mod ser;

pub mod connection;

use std::collections::{hash_map, HashSet};
use std::marker::PhantomData;
use std::slice;

use fnv::FnvHashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    port::{PortPins, WeakPortPins},
    ComponentId, ComponentRefId, Module, PortId,
};

use connection::Connection;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentData {
    pub name: String,
    pub(crate) ports: Vec<PortId>,
    pub(crate) references: Vec<ComponentRefId>,
    pub(crate) connections: Vec<Connection>,
    pub class: Option<ComponentClass>,
}

impl ComponentData {
    fn new(name: &str, class: Option<ComponentClass>) -> Self {
        Self {
            name: name.to_string(),
            ports: Vec::new(),
            references: Vec::new(),
            connections: Vec::new(),
            class,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ComponentKey(pub(crate) ComponentId);

impl ComponentKey {
    pub(crate) fn new(component: ComponentId) -> Self {
        Self(component)
    }

    pub fn promote(self, module: &Module) -> Component<'_> {
        Component::new(module, self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Component<'m>(&'m Module, ComponentId);

impl<'m> Component<'m> {
    pub(crate) fn new(module: &'m Module, component: ComponentId) -> Self {
        Self(module, component)
    }

    pub fn module(&self) -> &'m Module {
        self.0
    }

    pub fn key(&self) -> ComponentKey {
        ComponentKey::new(self.1)
    }

    pub(crate) fn data(&self) -> &'m ComponentData {
        &self.module()[self.1]
    }

    pub fn name(&self) -> &'m str {
        &self.data().name
    }

    // pub fn ports(&self) -> PortIter<'m> {
    //     PortIter {
    //         module: self.module,
    //         iter: self.data.ports.values(),
    //     }
    // }

    // pub fn references(&self) -> ComponentRefIter<'m> {
    //     ComponentRefIter {
    //         module: self.module,
    //         iter: self.data.references.iter(),
    //     }
    // }

    // pub fn connections(&self) -> ConnectionIter<'m> {
    //     ConnectionIter {
    //         iter: self.data.connections.iter(),
    //     }
    // }

    pub fn class(&self) -> Option<ComponentClass> {
        self.data().class
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRefData {
    pub(crate) component: ComponentId,
    pub alias: String,
    pub n_instances: usize,
}

impl ComponentRefData {
    pub(crate) fn new(component: ComponentId, alias: String, n_instances: usize) -> Self {
        Self {
            component,
            alias,
            n_instances,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ComponentRefKey(ComponentRefId);

impl ComponentRefKey {
    pub(crate) fn new(reference: ComponentRefId) -> Self {
        Self(reference)
    }

    pub fn promote(self, module: &Module) -> ComponentRef<'_> {
        ComponentRef::new(module, self.0)
    }
}

#[derive(Clone, Debug)]
pub struct ComponentRef<'m>(&'m Module, ComponentRefId);

impl<'m> ComponentRef<'m> {
    pub(crate) fn new(module: &'m Module, reference: ComponentRefId) -> Self {
        Self(module, reference)
    }

    pub(crate) fn module(&self) -> &'m Module {
        self.0
    }

    pub fn key(&self) -> ComponentRefKey {
        ComponentRefKey::new(self.1)
    }

    pub(crate) fn data(&self) -> &'m ComponentRefData {
        &self.module()[self.1]
    }

    pub fn component(&self) -> Component<'m> {
        Component::new(self.module(), self.data().component)
    }

    pub fn alias(&self) -> &'m str {
        &self.data().alias
    }

    pub fn n_instances(&self) -> usize {
        self.data().n_instances
    }
}

// impl<'m> PartialEq<ComponentRef<'m>> for &Component<'m> {
//     fn eq(&self, other: &ComponentRef) -> bool {
//         self.id == other.data.component
//     }
// }

// impl<'m> Index<PortId> for Component<'m> {
//     type Output = PortData;

//     fn index(&self, port: PortId) -> &Self::Output {
//         &self.module[port]
//     }
// }

pub struct PortIter<'m> {
    module: &'m Module,
    iter: hash_map::Values<'m, String, PortId>,
}

// impl<'m> Iterator for PortIter<'m> {
//     type Item = Port<'m>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let port = *self.iter.next()?;
//         Some(port.to_port(self.module))
//     }
// }

pub struct ComponentRefIter<'m> {
    module: &'m Module,
    iter: hash_map::Iter<'m, String, ComponentRefId>,
}

// impl<'m> Iterator for ComponentRefIter<'m> {
//     type Item = (&'m str, ComponentRef<'m>);

//     fn next(&mut self) -> Option<Self::Item> {
//         let (&alias, &reference) = self.iter.next()?;
//         let alias = &self.module.strings[alias];
//         let reference = ComponentRef::new(self.module, reference);
//         Some((alias, reference))
//     }
// }

pub struct ConnectionIter<'m> {
    iter: slice::Iter<'m, Connection>,
}

impl<'m> Iterator for ConnectionIter<'m> {
    type Item = &'m Connection;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub struct NameSet(String);
pub struct NameUnset;

pub struct ComponentBuilder<'m, N> {
    pub(crate) module: &'m mut Module,
    // pub(crate) placeholder: ComponentId,
    // pub(crate) data: ComponentData,
    // unresolved_references: HashSet<ComponentWeakRef>,
    // unresolved_named_references: FnvHashMap<String, ComponentWeakRef>,
    // unresolved_connections: Vec<WeakConnection>,
    name: N,
    class: Option<ComponentClass>,
}

impl<'m> ComponentBuilder<'m, NameUnset> {
    pub fn new(module: &'m mut Module) -> Self {
        Self {
            module,
            name: NameUnset,
            class: None,
        }
    }

    pub fn set_name(self, name: &str) -> ComponentBuilder<'m, NameSet> {
        ComponentBuilder {
            module: self.module,
            name: NameSet(name.to_string()),
            class: self.class,
        }
    }
}

impl<'m, N> ComponentBuilder<'m, N> {
    pub fn set_class(&mut self, class: ComponentClass) {
        self.class = Some(class)
    }

    pub fn class_is_set(&self) -> bool {
        self.class.is_some()
    }
}

impl<'m> ComponentBuilder<'m, NameSet> {
    pub fn finish(self) -> Component<'m> {
        let component = {
            let component = ComponentData::new(&self.name.0, self.class);
            self.module.components.insert(component)
        };

        Component::new(self.module, component)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ComponentWeakRef {
    pub(crate) component: String,
    pub(crate) alias: Option<String>,
    pub(crate) n_instances: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WeakConnection {
    pub(crate) source_pins: WeakPortPins,
    pub(crate) source_component: Option<String>,
    pub(crate) sink_pins: WeakPortPins,
    pub(crate) sink_component: Option<String>,
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

#[derive(Debug)]
pub struct ComponentBuildArtifacts {
    pub references: HashSet<ComponentWeakRef>,
    pub named_references: FnvHashMap<String, ComponentWeakRef>,
    pub connections: HashSet<WeakConnection>,
}

pub(crate) type ComponentBuildResult =
    Result<(ComponentId, ComponentBuildArtifacts), ComponentBuildError>;

impl<'m, N> ComponentBuilder<'m, N> {
    // pub fn new(module: &'m mut Module) -> Self {
    //     let data = ComponentData::new(module, "", None);
    //     let placeholder = module.component_db.entry(data.clone());
    //     assert!(module.components.insert(data.name, placeholder).is_none());

    //     let unresolved_references = HashSet::new();
    //     let unresolved_named_references = HashMap::new();
    //     let unresolved_connections = Vec::new();

    //     Self {
    //         module,
    //         data,
    //         placeholder,
    //         unresolved_references,
    //         unresolved_named_references,
    //         unresolved_connections,
    //         name_is_set: false,
    //     }
    // }

    // pub fn set_name(&mut self, name: &str) -> &mut Self {
    //     let name = self.module.strings.entry(name);
    //     assert!(
    //         self.module.components.get(&name).is_none(),
    //         r#"component "{component}" already in module "{module}""#,
    //         component = &self.module.strings[name],
    //         module = &self.module.strings[self.module.name]
    //     );

    //     if let Some(component) = self.module.components.remove(&self.data.name) {
    //         self.module.components.insert(name, component);
    //     } else {
    //         panic!("component not in module")
    //     }

    //     self.data.name = name;

    //     self.name_is_set = true;
    //     self
    // }

    // pub fn add_port(&mut self) -> PortBuilder<'_> {
    //     PortBuilder::new(self.module, self.placeholder)
    // }

    // pub fn add_reference(
    //     &mut self,
    //     component: ComponentId,
    //     alias: Option<&str>,
    //     n_instances: Option<usize>,
    // ) -> Result<ComponentRefId, ComponentBuildError> {
    //     let alias = match alias {
    //         Some(alias) => self.module.strings.entry(alias),
    //         None => self.module[component].name,
    //     };
    //     let n_instances = n_instances.unwrap_or(1);

    //     let reference = ComponentRefData::new(component, alias, n_instances);
    //     let reference = self.module.reference_db.entry(reference);
    //     if self.data.references.insert(alias, reference).is_some() {
    //         let component = self.module.strings[self.module[component].name].to_string();
    //         let reference = self.module.strings[alias].to_string();
    //         Err(ComponentBuildError::DuplicateReference {
    //             component,
    //             reference,
    //         })
    //     } else {
    //         Ok(reference)
    //     }
    // }

    // pub fn add_weak_reference<S: GetStringId>(
    //     &mut self,
    //     component: S,
    //     alias: Option<S>,
    //     n_instances: Option<usize>,
    // ) -> Result<ComponentWeakRef, ComponentBuildError> {
    //     let component = component.get_string_id(self.module);
    //     let alias = alias.map(|alias| alias.get_string_id(self.module));

    //     let reference = ComponentWeakRef {
    //         component,
    //         alias,
    //         n_instances: n_instances.unwrap_or(1),
    //     };

    //     if let Some(alias) = alias {
    //         if self
    //             .unresolved_named_references
    //             .insert(alias, reference)
    //             .is_some()
    //         {
    //             let parent = self.module.strings[self.data.name].to_string();
    //             let reference = self.module.strings[alias].to_string();
    //             return Err(ComponentBuildError::DuplicateReference {
    //                 component: parent,
    //                 reference,
    //             });
    //         }
    //     } else if !self.unresolved_references.insert(reference) {
    //         let parent = self.module.strings[self.data.name].to_string();
    //         let reference = self.module.strings[component].to_string();
    //         return Err(ComponentBuildError::DuplicateReference {
    //             component: parent,
    //             reference,
    //         });
    //     }

    //     Ok(reference)
    // }

    // pub fn add_connection(&mut self) -> ConnectionBuilder<'_> {
    //     ConnectionBuilder::new(&mut self.data)
    // }

    // pub fn add_weak_connection(&mut self) -> WeakConnectionBuilder<'_, 'm> {
    //     WeakConnectionBuilder::new(self)
    // }

    // pub fn set_class(&mut self, class: ComponentClass) -> &mut Self {
    //     self.data.class = Some(class);
    //     self
    // }

    // pub fn is_name_set(&self) -> bool {
    //     self.name_is_set
    // }

    // pub fn is_ports_empty(&self) -> bool {
    //     self.data.ports.is_empty()
    // }

    // pub fn is_references_empty(&self) -> bool {
    //     self.data.references.is_empty()
    // }

    // pub fn is_unresolved_references_empty(&self) -> bool {
    //     self.unresolved_references.is_empty()
    // }

    // pub fn is_unresolved_named_references_empty(&self) -> bool {
    //     self.unresolved_named_references.is_empty()
    // }

    // pub fn is_connections_empty(&self) -> bool {
    //     self.data.connections.is_empty()
    // }

    // pub fn is_class_set(&self) -> bool {
    //     self.data.class.is_some()
    // }

    // pub fn finish(mut self) -> ComponentBuildResult {
    //     use std::mem;

    //     if !self.is_name_set() {
    //         return Err(ComponentBuildError::MissingField("name"));
    //     }

    //     debug_assert!(self.data.ports.is_empty());
    //     mem::swap(
    //         &mut self.data.ports,
    //         &mut self.module[self.placeholder].ports,
    //     );

    //     // let name = self.data.name;
    //     // let component = self.module.component_db.entry(self.data);

    //     // if self.module.components.insert(name, component).is_some() {
    //     //     let component = self.module.strings[name].to_string();
    //     //     let module = self.module.strings[self.module.name].to_string();
    //     //     return Err(ComponentBuildError::DuplicateComponent { module, component });
    //     // }

    //     #[cfg(debug_assertions)]
    //     let name = self.data.name;

    //     self.module[self.placeholder] = self.data;

    //     #[cfg(debug_assertions)]
    //     {
    //         let component = self.module[self.placeholder].name(self.module);
    //         let component = self
    //             .module
    //             .strings
    //             .rlookup(component)
    //             .expect("component name should be in module strings");
    //         debug_assert_eq!(component, name);
    //         debug_assert!(self.module.components.contains_key(&component));
    //     }

    //     // TODO: check duplicate connections?
    //     let connections = HashSet::from_iter(self.unresolved_connections);

    //     Ok((
    //         self.placeholder,
    //         ComponentBuildArtifacts {
    //             references: self.unresolved_references,
    //             named_references: self.unresolved_named_references,
    //             connections,
    //         },
    //     ))
    // }
}
