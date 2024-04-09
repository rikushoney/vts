use super::{component::ComponentKey, Component, ComponentId, ComponentRefId, Module};

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRefData {
    pub(crate) component: ComponentId,
    alias: Option<String>,
    pub n_instances: usize,
}

impl ComponentRefData {
    pub(crate) fn new(component: ComponentId, alias: Option<String>, n_instances: usize) -> Self {
        Self {
            component,
            alias,
            n_instances,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentRefKey(pub(crate) ComponentRefId);

impl ComponentRefKey {
    pub(crate) fn new(reference: ComponentRefId) -> Self {
        Self(reference)
    }

    pub fn bind(self, module: &Module) -> ComponentRef<'_> {
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

    pub fn alias(&self) -> Option<&'m String> {
        self.data().alias.as_ref()
    }

    pub fn alias_or_name(&self) -> &'m str {
        if let Some(alias) = self.alias() {
            alias
        } else {
            self.component().name()
        }
    }

    pub fn n_instances(&self) -> usize {
        self.data().n_instances
    }
}

pub struct ComponentRefBuilder<'m> {
    module: &'m mut Module,
    parent: ComponentId,
    alias: Option<String>,
    n_instances: Option<usize>,
}

impl<'m> ComponentRefBuilder<'m> {
    pub fn new(module: &'m mut Module, parent: ComponentKey) -> Self {
        Self {
            module,
            parent: parent.0,
            alias: None,
            n_instances: None,
        }
    }

    pub fn set_alias(&mut self, alias: &str) {
        self.alias = Some(alias.to_string());
    }

    pub fn set_n_instances(&mut self, n_instances: usize) {
        self.n_instances = Some(n_instances);
    }

    pub fn alias_is_set(&self) -> bool {
        self.alias.is_some()
    }

    pub fn n_instances_is_set(&self) -> bool {
        self.n_instances.is_some()
    }

    pub fn finish(self) -> ComponentRef<'m> {
        let n_instances = self.n_instances.unwrap_or(1);

        let reference = {
            let reference = ComponentRefData::new(self.parent, self.alias, n_instances);
            self.module.references.insert(reference)
        };

        // TODO: check duplicate references

        self.module[self.parent].references.push(reference);

        ComponentRef::new(self.module, reference)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ComponentWeakRef {
    pub(crate) component: String,
    pub(crate) alias: Option<String>,
    pub(crate) n_instances: usize,
}
