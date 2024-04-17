use std::ops::Range;

use serde::Serialize;
use ustr::{ustr, Ustr};

use super::{
    checker,
    component::ComponentKey,
    connection::ComponentRefSelection,
    linker::{self, KnownComponents, Resolve},
    prelude::*,
};

pub(super) const FIELDS: &[&str] = &["component", "n_instances"];

pub(super) const COMPONENT: usize = 0;
pub(super) const N_INSTANCES: usize = 1;

pub(super) mod reference_range {
    pub const FIELDS: &[&str] = &["reference_start", "reference_end"];

    pub const REFERENCE_START: usize = 0;
    pub const REFERENCE_END: usize = 1;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRefData {
    pub(crate) component: ComponentId,
    pub(crate) parent: ComponentId,
    pub(crate) alias: Option<Ustr>,
    pub n_instances: u32,
}

impl ComponentRefData {
    pub(crate) fn new(
        component: ComponentId,
        parent: ComponentId,
        alias: Option<Ustr>,
        n_instances: u32,
    ) -> Self {
        Self {
            component,
            parent,
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

    pub fn parent(&self) -> Component<'m> {
        Component::new(self.module(), self.data().parent)
    }

    pub fn alias(&self) -> Option<&'m str> {
        self.data().alias.as_ref().map(Ustr::as_str)
    }

    pub fn alias_or_name(&self) -> &'m str {
        if let Some(alias) = self.alias() {
            alias
        } else {
            self.component().name()
        }
    }

    pub fn n_instances(&self) -> u32 {
        self.data().n_instances
    }

    #[must_use]
    pub fn select(&self, range: ReferenceRange) -> ComponentRefSelection {
        ComponentRefSelection::new(self.key(), range)
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum ReferenceRange {
    Start(u32),
    End(u32),
    Bound(Range<u32>),
    #[default]
    Full,
}

impl ReferenceRange {
    pub fn new(start: Option<u32>, end: Option<u32>) -> Self {
        match (start, end) {
            (Some(start), Some(end)) => Self::Bound(Range { start, end }),
            (Some(start), None) => Self::Start(start),
            (None, Some(end)) => Self::End(end),
            (None, None) => Self::Full,
        }
    }

    pub fn get_start(&self) -> Option<u32> {
        match self {
            Self::Start(start) => Some(*start),
            Self::Bound(Range { start, .. }) => Some(*start),
            _ => None,
        }
    }

    pub fn get_end(&self) -> Option<u32> {
        match self {
            Self::End(end) => Some(*end),
            Self::Bound(Range { end, .. }) => Some(*end),
            _ => None,
        }
    }

    #[must_use]
    pub fn expand(&self, n_instances: u32) -> Range<u32> {
        match self {
            Self::Start(start) => Range {
                start: *start,
                end: n_instances,
            },
            Self::End(end) => Range {
                start: 0,
                end: *end,
            },
            Self::Bound(range) => range.clone(),
            Self::Full => Range {
                start: 0,
                end: n_instances,
            },
        }
    }

    pub fn flatten(&mut self, n_instances: u32) {
        match self {
            Self::Start(start) => {
                if *start == 0 {
                    *self = Self::Full;
                }
            }
            Self::End(end) => {
                if *end == n_instances {
                    *self = Self::Full;
                }
            }
            Self::Bound(range) => {
                if range.start == 0 {
                    *self = Self::End(range.end);
                    return self.flatten(n_instances);
                }

                if range.end == n_instances {
                    *self = Self::Start(range.start);
                    self.flatten(n_instances)
                }
            }
            Self::Full => {}
        }
    }
}

pub struct ComponentSet(ComponentId);
pub struct ComponentUnset;

pub struct ComponentRefBuilder<'a, 'm, C> {
    module: &'m mut Module,
    checker: &'a mut Checker,
    parent: ComponentId,
    component: C,
    alias: Option<Ustr>,
    n_instances: Option<u32>,
}

impl<'a, 'm> ComponentRefBuilder<'a, 'm, ComponentUnset> {
    pub fn new(module: &'m mut Module, checker: &'a mut Checker, parent: ComponentKey) -> Self {
        Self {
            module,
            checker,
            parent: parent.0,
            component: ComponentUnset,
            alias: None,
            n_instances: None,
        }
    }

    pub fn set_component(
        self,
        component: ComponentKey,
    ) -> ComponentRefBuilder<'a, 'm, ComponentSet> {
        ComponentRefBuilder {
            module: self.module,
            checker: self.checker,
            parent: self.parent,
            component: ComponentSet(component.0),
            alias: self.alias,
            n_instances: self.n_instances,
        }
    }
}

impl<'a, 'm, C> ComponentRefBuilder<'a, 'm, C> {
    pub fn set_alias(&mut self, alias: &str) {
        self.alias = Some(ustr(alias));
    }

    pub fn set_n_instances(&mut self, n_instances: u32) {
        self.n_instances = Some(n_instances);
    }

    pub fn alias_is_set(&self) -> bool {
        self.alias.is_some()
    }

    pub fn n_instances_is_set(&self) -> bool {
        self.n_instances.is_some()
    }
}

impl<'a, 'm> ComponentRefBuilder<'a, 'm, ComponentSet> {
    fn insert(&mut self) -> ComponentRefId {
        let n_instances = self.n_instances.unwrap_or(1);
        let reference = ComponentRefData::new(
            self.component.0,
            self.parent,
            self.alias.take(),
            n_instances,
        );
        self.module.references.insert(reference)
    }

    pub fn finish(mut self) -> Result<ComponentRef<'m>, checker::Error> {
        let reference = {
            let reference = {
                let reference = self.insert();
                ComponentRef::new(self.module, reference)
            };

            self.checker.register_reference(
                self.module,
                reference.component().key(),
                reference.key(),
            )?;
            reference.1
        };

        self.module[self.parent].references.push(reference);
        Ok(ComponentRef::new(self.module, reference))
    }
}

fn equals_one(x: &u32) -> bool {
    *x == 1
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct ComponentWeakRef {
    pub component: Ustr,
    #[serde(skip)]
    pub alias: Option<Ustr>,
    #[serde(skip_serializing_if = "equals_one")]
    pub n_instances: u32,
}

impl ComponentWeakRef {
    pub fn alias_or_name(&self) -> &str {
        if let Some(alias) = self.alias {
            alias
        } else {
            self.component
        }
        .as_str()
    }
}

impl<'a, 'm> Resolve<'a, 'm> for ComponentWeakRef {
    type Output = ComponentRefKey;

    fn resolve(
        self,
        module: &mut Module,
        checker: &mut Checker,
        component: ComponentKey,
        components: &KnownComponents,
    ) -> Result<Self::Output, linker::Error> {
        let referenced_component = {
            let component = self.component.as_str();
            components.get(module, component)?.key()
        };

        let mut builder = ComponentRefBuilder::new(module, checker, component)
            .set_component(referenced_component);

        if let Some(alias) = &self.alias {
            builder.set_alias(alias);
        }

        Ok(builder.finish().map_err(linker::Error::from)?.key())
    }
}
