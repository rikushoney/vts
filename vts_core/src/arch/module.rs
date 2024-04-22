use slotmap::{new_key_type, SlotMap};
use ustr::{ustr, Ustr};

use super::prelude::*;

pub(super) const FIELDS: &[&str] = &["name", "components"];

pub(super) const NAME: usize = 0;
pub(super) const COMPONENTS: usize = 1;

new_key_type! {
    pub struct ComponentId;
    pub struct ComponentRefId;
    pub struct PortId;
    pub struct ConnectionId;
}

#[derive(Clone, Debug)]
pub struct Module {
    pub(crate) name: Ustr,
    pub(crate) components: SlotMap<ComponentId, ComponentData>,
    pub(crate) ports: SlotMap<PortId, PortData>,
    pub(crate) references: SlotMap<ComponentRefId, ComponentRefData>,
    pub(crate) connections: SlotMap<ConnectionId, ConnectionData>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: ustr(name),
            components: SlotMap::default(),
            ports: SlotMap::default(),
            references: SlotMap::default(),
            connections: SlotMap::default(),
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

    pub fn get_component(&self, component: ComponentId) -> Option<Component<'_>> {
        self.components.get(component).map(|_| component.bind(self))
    }

    pub fn get_port(&self, port: PortId) -> Option<Port<'_>> {
        self.ports.get(port).map(|_| port.bind(self))
    }

    pub fn get_reference(&self, reference: ComponentRefId) -> Option<ComponentRef<'_>> {
        self.references
            .get(reference.id())
            .map(|_| reference.bind(self))
    }

    pub fn find_component(&self, name: &str) -> Option<Component<'_>> {
        self.components
            .iter()
            .find(|(_, component)| component.name == name)
            .map(|(component, _)| component.bind(self))
    }
}

pub(crate) trait ModuleLookup<I> {
    type Output;

    fn lookup(&self, id: I) -> &Self::Output;
    fn lookup_mut(&mut self, id: I) -> &mut Self::Output;
}

macro_rules! impl_module_lookup_ops {
    ($($id:ident => $data:ident in $db:ident),+ $(,)?) => {
        $(
            impl ModuleLookup<$id> for Module {
                type Output = $data;

                fn lookup(&self, id: $id) -> &Self::Output {
                    &self.$db[id]
                }

                fn lookup_mut(&mut self, id: $id) -> &mut Self::Output {
                    &mut self.$db[id]
                }
            }
        )+
    }
}

impl_module_lookup_ops!(
    ComponentId => ComponentData in components,
    PortId => PortData in ports,
    ComponentRefId => ComponentRefData in references,
    ConnectionId => ConnectionData in connections,
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
            .map(|component| component.bind(self.module))
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
