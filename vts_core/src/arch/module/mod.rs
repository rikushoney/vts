#![allow(unused)] // TODO: remove this!

// pub mod de;
// pub mod ser;

use std::collections::hash_map;

use fnv::FnvHashMap;
use slotmap::{new_key_type, SlotMap};
use thiserror::Error;

use super::component::{
    ComponentBuildArtifacts, ComponentData, ComponentRefData, ConnectionBuildError, WeakConnection,
};
use super::port::PortData;

new_key_type! {
    pub(crate) struct ComponentId;
    pub(crate) struct ComponentRefId;
    pub(crate) struct PortId;
}

#[derive(Clone, Debug)]
pub struct Module {
    pub(crate) name: String,
    pub(crate) components: SlotMap<ComponentId, ComponentData>,
    pub(crate) component_names: FnvHashMap<String, ComponentId>,
    pub(crate) ports: SlotMap<PortId, PortData>,
    pub(crate) references: SlotMap<ComponentRefId, ComponentRefData>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            components: SlotMap::default(),
            component_names: FnvHashMap::default(),
            ports: SlotMap::default(),
            references: SlotMap::default(),
        }
    }

    // pub fn name(&self) -> &str {
    //     &self.strings[self.name]
    // }

    // pub fn rename(&mut self, name: &str) {
    //     self.name = self.strings.entry(name);
    // }

    pub fn components(&self) -> ComponentIter {
        ComponentIter {
            module: self,
            iter: self.component_names.values(),
        }
    }

    // pub fn get_component(&self, component: ComponentId) -> Option<&ComponentData> {
    //     if self.components.values().any(|c| c == &component) {
    //         Some(&self[component])
    //     } else {
    //         None
    //     }
    // }

    // pub fn get_component_mut(&mut self, component: ComponentId) -> Option<&mut ComponentData> {
    //     if self.components.values().any(|c| c == &component) {
    //         Some(&mut self[component])
    //     } else {
    //         None
    //     }
    // }
}

macro_rules! impl_module_index_ops {
    ($($id:ident => $data:ident in $db:ident),+ $(,)?) => {
        $(
            impl Index<$id> for Module {
                type Output = $data;

                fn index(&self, id: $id) -> &Self::Output {
                    &self.$db[id]
                }
            }

            impl IndexMut<$id> for Module {
                fn index_mut(&mut self, id: $id) -> &mut Self::Output {
                    &mut self.$db[id]
                }
            }
        )+
    }
}

// impl_module_index_ops!(
//     ComponentId => ComponentData in component_db,
//     PortId => PortData in port_db,
//     ComponentRefId => ComponentRefData in reference_db
// );

pub struct ComponentIter<'m> {
    module: &'m Module,
    iter: hash_map::Values<'m, String, ComponentId>,
}

// impl<'m> Iterator for ComponentIter<'m> {
//     type Item = Component<'m>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let component = *self.iter.next()?;
//         Some(component.to_component(self.module))
//     }
// }

pub struct ModuleBuilder {
    module: Module,
    name_is_set: bool,
    // references_are_resolved: bool,
    // connections_are_resolved: bool,
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum ModuleBuildError {
    #[error("{0}")]
    Connection(#[from] ConnectionBuildError),
    #[error(r#"component "{reference}" already referenced in "{component}""#)]
    DuplicateReference {
        component: String,
        reference: String,
    },
    #[error("module must have a {0}")]
    MissingField(&'static str),
    #[error(r#"undefined component "{reference}" referenced in "{component}""#)]
    UndefinedReference {
        component: String,
        reference: String,
    },
}

// pub trait ReferenceResolve {
//     fn get_alias(
//         &self,
//         module: &Module,
//         component: ComponentId,
//     ) -> Result<StringId, ModuleBuildError>;

//     fn get_weak_reference(&self) -> &ComponentWeakRef;

//     fn resolve(
//         &self,
//         module: &Module,
//         component: ComponentId,
//     ) -> Result<(StringId, ComponentRefData), ModuleBuildError> {
//         let reference = self.get_weak_reference();
//         if let Some(&component) = module.components.get(&reference.component) {
//             let alias = self.get_alias(module, component)?;
//             let n_instances = reference.n_instances;
//             let reference = ComponentRefData::new(component, alias, n_instances);
//             Ok((alias, reference))
//         } else {
//             let reference = module.strings[reference.component].to_string();
//             let component = module[component].name(module).to_string();
//             Err(ModuleBuildError::UndefinedReference {
//                 component,
//                 reference,
//             })
//         }
//     }
// }

// impl ReferenceResolve for ComponentWeakRef {
//     fn get_alias(
//         &self,
//         module: &Module,
//         component: ComponentId,
//     ) -> Result<StringId, ModuleBuildError> {
//         module
//             .components
//             .get(&self.component)
//             .map(|&component| module.component_db[component].name)
//             .ok_or_else(|| {
//                 let reference = module.strings[self.component].to_string();
//                 let component = module[component].name(module).to_string();
//                 ModuleBuildError::UndefinedReference {
//                     component,
//                     reference,
//                 }
//             })
//     }

//     fn get_weak_reference(&self) -> &ComponentWeakRef {
//         self
//     }
// }

// impl ReferenceResolve for (StringId, ComponentWeakRef) {
//     fn get_alias(
//         &self,
//         _module: &Module,
//         _component: ComponentId,
//     ) -> Result<StringId, ModuleBuildError> {
//         Ok(self.0)
//     }

//     fn get_weak_reference(&self) -> &ComponentWeakRef {
//         &self.1
//     }
// }

// pub trait ConnectionResolve {
//     fn get_source_component(
//         &self,
//         module: &Module,
//         parent: ComponentId,
//     ) -> Result<ComponentId, ConnectionBuildError>;

//     fn resolve_source_reference(
//         &self,
//         module: &mut Module,
//         parent: ComponentId,
//     ) -> Result<Option<ComponentRefId>, ConnectionBuildError>;

//     fn get_source_pins(&self) -> &WeakPortPins;

//     fn get_sink_component(
//         &self,
//         module: &Module,
//         parent: ComponentId,
//     ) -> Result<ComponentId, ConnectionBuildError>;

//     fn resolve_sink_reference(
//         &self,
//         module: &mut Module,
//         parent: ComponentId,
//     ) -> Result<Option<ComponentRefId>, ConnectionBuildError>;

//     fn get_sink_pins(&self) -> &WeakPortPins;

//     fn resolve_source_pins(
//         &self,
//         module: &mut Module,
//         parent: ComponentId,
//     ) -> Result<PortPins, ConnectionBuildError> {
//         let component = self.get_source_component(module, parent)?;
//         let component = &module.component_db[component];

//         let source_pins = self.get_source_pins();
//         let port = source_pins.port;
//         let &port = component
//             .ports
//             .get(&port)
//             .ok_or(ConnectionBuildError::UndefinedPort {
//                 port: module.strings[port].to_string(),
//             })?;
//         let port = port.to_port(module);

//         Ok(port.select(source_pins.range.clone()))
//     }

//     fn resolve_sink_pins(
//         &self,
//         module: &mut Module,
//         parent: ComponentId,
//     ) -> Result<PortPins, ConnectionBuildError> {
//         let component = self.get_sink_component(module, parent)?;
//         let component = &module.component_db[component];

//         let sink_pins = self.get_sink_pins();
//         let port = sink_pins.port;
//         let &port = component
//             .ports
//             .get(&port)
//             .ok_or(ConnectionBuildError::UndefinedPort {
//                 port: module.strings[port].to_string(),
//             })?;
//         let port = port.to_port(module);

//         Ok(port.select(sink_pins.range.clone()))
//     }
// }

// impl ConnectionResolve for WeakConnection {
//     fn get_source_component(
//         &self,
//         module: &Module,
//         parent: ComponentId,
//     ) -> Result<ComponentId, ConnectionBuildError> {
//         // TODO: is this ever `None`?
//         let component = self
//             .source_component
//             .ok_or(ConnectionBuildError::MissingField("source component"))?;

//         let reference = module.component_db[parent]
//             .references
//             .get(&component)
//             .ok_or(ConnectionBuildError::UndefinedReference {
//                 reference: module.strings[component].to_string(),
//             })
//             .copied()?;

//         Ok(module[reference].component)
//     }

//     fn resolve_source_reference(
//         &self,
//         module: &mut Module,
//         parent: ComponentId,
//     ) -> Result<Option<ComponentRefId>, ConnectionBuildError> {
//         let component = self
//             .source_component
//             .ok_or(ConnectionBuildError::MissingField("source component"))?;

//         let parent = &module.component_db[parent];
//         Ok(parent.references.get(&component).copied())
//     }

//     fn get_source_pins(&self) -> &WeakPortPins {
//         &self.source_pins
//     }

//     fn get_sink_component(
//         &self,
//         module: &Module,
//         parent: ComponentId,
//     ) -> Result<ComponentId, ConnectionBuildError> {
//         // TODO: is this ever `None`?
//         let component = self
//             .sink_component
//             .ok_or(ConnectionBuildError::MissingField("sink component"))?;

//         let reference = module.component_db[parent]
//             .references
//             .get(&component)
//             .ok_or(ConnectionBuildError::UndefinedReference {
//                 reference: module.strings[component].to_string(),
//             })
//             .copied()?;

//         Ok(module[reference].component)
//     }

//     fn resolve_sink_reference(
//         &self,
//         module: &mut Module,
//         parent: ComponentId,
//     ) -> Result<Option<ComponentRefId>, ConnectionBuildError> {
//         let component = self
//             .sink_component
//             .ok_or(ConnectionBuildError::MissingField("sink component"))?;

//         let parent = &module.component_db[parent];
//         Ok(parent.references.get(&component).copied())
//     }

//     fn get_sink_pins(&self) -> &WeakPortPins {
//         &self.sink_pins
//     }
// }

// impl ConnectionResolve for (&WeakConnection, ComponentId) {
//     fn get_source_component(
//         &self,
//         _module: &Module,
//         parent: ComponentId,
//     ) -> Result<ComponentId, ConnectionBuildError> {
//         debug_assert!(parent == self.1);
//         Ok(self.1)
//     }

//     fn resolve_source_reference(
//         &self,
//         _module: &mut Module,
//         _parent: ComponentId,
//     ) -> Result<Option<ComponentRefId>, ConnectionBuildError> {
//         Ok(None)
//     }

//     fn get_source_pins(&self) -> &WeakPortPins {
//         &self.0.source_pins
//     }

//     fn get_sink_component(
//         &self,
//         _module: &Module,
//         parent: ComponentId,
//     ) -> Result<ComponentId, ConnectionBuildError> {
//         debug_assert!(parent == self.1);
//         Ok(self.1)
//     }

//     fn resolve_sink_reference(
//         &self,
//         _module: &mut Module,
//         _parent: ComponentId,
//     ) -> Result<Option<ComponentRefId>, ConnectionBuildError> {
//         Ok(None)
//     }

//     fn get_sink_pins(&self) -> &WeakPortPins {
//         &self.0.sink_pins
//     }
// }

impl ModuleBuilder {
    pub fn new() -> Self {
        let module = Module::new("");

        Self {
            module,
            name_is_set: false,
            // references_are_resolved: false,
            // connections_are_resolved: false,
        }
    }

    // pub fn set_name(&mut self, name: &str) -> &mut Self {
    //     self.module.rename(name);
    //     self.name_is_set = true;
    //     self
    // }

    // pub fn add_component(&mut self) -> ComponentBuilder<'_> {
    //     ComponentBuilder::new(&mut self.module)
    // }

    // pub fn resolve_references<I, R>(
    //     &mut self,
    //     component: ComponentId,
    //     references: I,
    // ) -> Result<&mut Self, ModuleBuildError>
    // where
    //     I: Iterator<Item = R>,
    //     R: ReferenceResolve,
    // {
    //     // if self.references_are_resolved {
    //     //     return Ok(self);
    //     // }

    //     let module = &mut self.module;

    //     for reference in references {
    //         let (alias, reference) = reference.resolve(module, component)?;
    //         let reference = module.reference_db.entry(reference);
    //         if module[component]
    //             .references
    //             .insert(alias, reference)
    //             .is_some()
    //         {
    //             let component = module.component_db[component].name(module).to_string();
    //             let reference = module.strings[alias].to_string();
    //             return Err(ModuleBuildError::DuplicateReference {
    //                 component,
    //                 reference,
    //             });
    //         }
    //     }

    //     // self.references_are_resolved = true;
    //     Ok(self)
    // }

    pub(crate) fn resolve_connections<I: Iterator<Item = WeakConnection>>(
        &mut self,
        component: ComponentId,
        connections: I,
    ) -> Result<&mut Self, ModuleBuildError> {
        // if self.connections_are_resolved {
        //     return Ok(self);
        // }

        for connection in connections {
            // let (source_pins, source_reference) = if connection.source_component.is_some() {
            //     (
            //         connection.resolve_source_pins(&mut self.module, component)?,
            //         connection.resolve_source_reference(&mut self.module, component)?,
            //     )
            // } else {
            //     (
            //         (&connection, component).resolve_source_pins(&mut self.module, component)?,
            //         (&connection, component)
            //             .resolve_source_reference(&mut self.module, component)?,
            //     )
            // };

            // let (sink_pins, sink_reference) = if connection.sink_component.is_some() {
            //     (
            //         connection.resolve_sink_pins(&mut self.module, component)?,
            //         connection.resolve_sink_reference(&mut self.module, component)?,
            //     )
            // } else {
            //     (
            //         (&connection, component).resolve_sink_pins(&mut self.module, component)?,
            //         (&connection, component).resolve_sink_reference(&mut self.module, component)?,
            //     )
            // };

            // let connection =
            //     Connection::new(source_pins, sink_pins, source_reference, sink_reference);

            // self.module[component].connections.push(connection);
        }

        // self.connections_are_resolved = true;
        Ok(self)
    }

    // pub fn resolve_all<R, Rs, Ns, Cs>(
    //     &mut self,
    //     component: ComponentId,
    //     references: Rs,
    //     named_references: Ns,
    //     connections: Cs,
    // ) -> Result<(), ModuleBuildError>
    // where
    //     R: ReferenceResolve,
    //     Rs: IntoIterator<Item = R>,
    //     Ns: IntoIterator<Item = (StringId, ComponentWeakRef)>,
    //     Cs: IntoIterator<Item = WeakConnection>,
    // {
    //     let references = references.into_iter();
    //     let named_references = named_references.into_iter();
    //     let connections = connections.into_iter();

    //     // assert_eq!(references.size_hint(), named_references.size_hint());
    //     // assert_eq!(references.size_hint(), connections.size_hint());

    //     self.resolve_references(component, references)?;
    //     self.resolve_references(component, named_references)?;
    //     self.resolve_connections(component, connections)?;

    //     Ok(())
    // }

    pub fn is_name_set(&self) -> bool {
        self.name_is_set
    }

    pub fn is_components_empty(&self) -> bool {
        self.module.components.is_empty()
    }

    // pub fn are_references_resolved(&self) -> bool {
    //     self.references_are_resolved
    // }

    // pub fn are_connections_resolved(&self) -> bool {
    //     self.connections_are_resolved
    // }

    pub fn finish(self) -> Result<Module, ModuleBuildError> {
        if !self.is_name_set() {
            return Err(ModuleBuildError::MissingField("name"));
        }

        Ok(self.module)
    }

    pub(crate) fn resolve_and_finish<I>(mut self, unresolved: I) -> Result<Module, ModuleBuildError>
    where
        I: IntoIterator<Item = (ComponentId, ComponentBuildArtifacts)>,
    {
        for (component, artifacts) in unresolved.into_iter() {
            // self.resolve_all(
            //     component,
            //     artifacts.references,
            //     artifacts.named_references,
            //     artifacts.connections,
            // )?;
        }

        self.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module() {
        let mut _module = Module::new("test_mod");
    }
}
