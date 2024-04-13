use std::ops::{Index, IndexMut};

use slotmap::{new_key_type, SlotMap};
use ustr::{ustr, Ustr};

use super::{component::ComponentKey, port::PortKey, prelude::*, reference::ComponentRefKey};

pub(super) const FIELDS: &[&str] = &["name", "components"];

pub(super) const NAME: usize = 0;
pub(super) const COMPONENTS: usize = 1;

new_key_type! {
    pub(crate) struct ComponentId;
    pub(crate) struct ComponentRefId;
    pub(crate) struct PortId;
}

#[derive(Clone, Debug)]
pub struct Module {
    pub(crate) name: Ustr,
    pub(crate) components: SlotMap<ComponentId, ComponentData>,
    pub(crate) ports: SlotMap<PortId, PortData>,
    pub(crate) references: SlotMap<ComponentRefId, ComponentRefData>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: ustr(name),
            components: SlotMap::default(),
            ports: SlotMap::default(),
            references: SlotMap::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rename(&mut self, name: &str) {
        self.name = ustr(name);
    }

    pub fn components(&self) -> ComponentIter {
        ComponentIter {
            module: self,
            iter: self.components.keys(),
        }
    }

    pub fn get_component(&self, component: ComponentKey) -> Option<Component<'_>> {
        let component = component.0;

        self.components
            .get(component)
            .map(|_| Component::new(self, component))
    }

    pub fn get_component_data(&mut self, component: ComponentKey) -> Option<&mut ComponentData> {
        self.components.get_mut(component.0)
    }

    pub fn get_port(&self, port: PortKey) -> Option<Port<'_>> {
        let port = port.0;
        self.ports.get(port).map(|_| Port::new(self, port))
    }

    pub fn get_reference(&self, reference: ComponentRefKey) -> Option<ComponentRef<'_>> {
        let reference = reference.0;

        self.references
            .get(reference)
            .map(|_| ComponentRef::new(self, reference))
    }

    pub fn find_component(&self, name: &str) -> Option<Component<'_>> {
        self.components
            .iter()
            .find(|(_, component)| component.name == name)
            .map(|(component, _)| Component::new(self, component))
    }
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

impl_module_index_ops!(
    ComponentId => ComponentData in components,
    PortId => PortData in ports,
    ComponentRefId => ComponentRefData in references,
);

pub struct ComponentIter<'m> {
    module: &'m Module,
    iter: slotmap::basic::Keys<'m, ComponentId, ComponentData>,
}

impl<'m> Iterator for ComponentIter<'m> {
    type Item = Component<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|component| Component::new(self.module, component))
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
