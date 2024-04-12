use std::ops::Range;

use serde::{Deserialize, Serialize};

use super::{
    component::ComponentKey,
    linker::{self, KnownComponents, Resolve},
    prelude::*,
    reference::ComponentRefKey,
};

pub(super) const FIELDS: &[&str] = &["kind", "n_pins", "class"];

pub(super) const KIND: usize = 0;
pub(super) const N_PINS: usize = 1;
pub(super) const CLASS: usize = 2;

pub(super) mod pin_range {
    pub const FIELDS: &[&str] = &["port_start", "port_end"];

    pub const PORT_START: usize = 0;
    pub const PORT_END: usize = 1;
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortClass {
    Clock,
    LutIn,
    LutOut,
    LatchIn,
    LatchOut,
}

fn equals_one(x: &u32) -> bool {
    *x == 1
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PortData {
    #[serde(skip)]
    pub name: String,
    #[serde(skip)]
    parent: ComponentId,
    pub kind: PortKind,
    #[serde(skip_serializing_if = "equals_one")]
    pub n_pins: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<PortClass>,
}

impl PortData {
    pub(crate) fn new(
        parent: ComponentId,
        name: &str,
        kind: PortKind,
        n_pins: u32,
        class: Option<PortClass>,
    ) -> Self {
        Self {
            name: name.to_string(),
            parent,
            kind,
            n_pins,
            class,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PortKey(pub(crate) PortId);

impl PortKey {
    pub(crate) fn new(port: PortId) -> Self {
        Self(port)
    }

    pub fn bind(self, module: &Module) -> Port<'_> {
        Port::new(module, self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Port<'m>(&'m Module, PortId);

impl<'m> Port<'m> {
    pub(crate) fn new(module: &'m Module, port: PortId) -> Self {
        Self(module, port)
    }

    pub fn module(&self) -> &'m Module {
        self.0
    }

    pub fn key(&self) -> PortKey {
        PortKey::new(self.1)
    }

    pub fn parent(&self) -> Component<'_> {
        Component::new(self.0, self.data().parent)
    }

    pub fn name(&self) -> &str {
        &self.module()[self.1].name
    }

    pub(crate) fn data(&self) -> &'m PortData {
        &self.module().ports[self.1]
    }

    pub fn kind(&self) -> PortKind {
        self.data().kind
    }

    pub fn n_pins(&self) -> u32 {
        self.data().n_pins
    }

    pub fn class(&self) -> Option<PortClass> {
        self.data().class
    }

    #[must_use]
    pub fn select(&self, range: PinRange) -> PortPins {
        PortPins::new(self.1, range)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PinRange {
    Start(u32),
    End(u32),
    Bound(Range<u32>),
    Full,
}

impl PinRange {
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

    pub fn expand(&self, n_pins: u32) -> Range<u32> {
        match self {
            Self::Start(start) => Range {
                start: *start,
                end: n_pins,
            },
            Self::End(end) => Range {
                start: 0,
                end: *end,
            },
            Self::Bound(range) => range.clone(),
            Self::Full => Range {
                start: 0,
                end: n_pins,
            },
        }
    }

    pub fn flatten(&mut self, n_pins: u32) {
        match self {
            Self::Start(start) => {
                if *start == 0 {
                    *self = PinRange::Full;
                }
            }
            Self::End(end) => {
                if *end == n_pins {
                    *self = PinRange::Full;
                }
            }
            Self::Bound(range) => {
                if range.start == 0 {
                    *self = PinRange::End(range.end);
                    return self.flatten(n_pins);
                }

                if range.end == n_pins {
                    *self = PinRange::Start(range.start);
                    self.flatten(n_pins)
                }
            }
            Self::Full => {}
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PortPins {
    port: PortId,
    pub range: PinRange,
}

impl PortPins {
    pub(crate) fn new(port: PortId, range: PinRange) -> Self {
        Self { port, range }
    }

    pub fn port<'m>(&self, module: &'m Module) -> Port<'m> {
        Port::new(module, self.port)
    }
}

pub struct NameSet(String);
pub struct NameUnset;
pub struct KindSet(PortKind);
pub struct KindUnset;

pub struct PortBuilder<'m, N, K> {
    module: &'m mut Module,
    parent: ComponentId,
    name: N,
    kind: K,
    n_pins: Option<u32>,
    class: Option<PortClass>,
}

impl<'m> PortBuilder<'m, NameUnset, KindUnset> {
    pub fn new(module: &'m mut Module, component: ComponentKey) -> Self {
        Self {
            module,
            parent: component.0,
            name: NameUnset,
            kind: KindUnset,
            n_pins: None,
            class: None,
        }
    }
}

impl<'m, K> PortBuilder<'m, NameUnset, K> {
    pub fn set_name(self, name: &str) -> PortBuilder<'m, NameSet, K> {
        PortBuilder {
            module: self.module,
            parent: self.parent,
            name: NameSet(name.to_string()),
            kind: self.kind,
            n_pins: self.n_pins,
            class: self.class,
        }
    }
}

impl<'m, N> PortBuilder<'m, N, KindUnset> {
    pub fn set_kind(self, kind: PortKind) -> PortBuilder<'m, N, KindSet> {
        PortBuilder {
            module: self.module,
            parent: self.parent,
            name: self.name,
            kind: KindSet(kind),
            n_pins: self.n_pins,
            class: self.class,
        }
    }
}

impl<'m, N, K> PortBuilder<'m, N, K> {
    pub fn set_n_pins(&mut self, n_pins: u32) {
        self.n_pins = Some(n_pins);
    }

    pub fn set_class(&mut self, class: PortClass) {
        self.class = Some(class);
    }

    pub fn n_pins_is_set(&self) -> bool {
        self.n_pins.is_some()
    }

    pub fn class_is_set(&self) -> bool {
        self.class.is_some()
    }
}

impl<'m> PortBuilder<'m, NameSet, KindSet> {
    fn insert(&mut self) -> PortId {
        // TODO: check duplicate ports
        let port = PortData::new(
            self.parent,
            &self.name.0,
            self.kind.0,
            self.n_pins.unwrap_or(1),
            self.class,
        );

        self.module.ports.insert(port)
    }

    pub fn finish(mut self) -> Port<'m> {
        let port = self.insert();
        self.module[self.parent].ports.push(port);
        Port::new(self.module, port)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct WeakPortPins {
    pub port: String,
    #[serde(flatten)]
    pub range: PinRange,
}

impl<'m> Resolve<'m> for (WeakPortPins, Option<ComponentRefKey>) {
    type Output = PortPins;

    fn resolve(
        self,
        module: &'m mut Module,
        parent: ComponentKey,
        _components: &KnownComponents,
    ) -> Result<Self::Output, linker::Error> {
        let component = Component::new(module, parent.0);

        let parent = if let Some(reference) = self.1 {
            ComponentRef::new(module, reference.0).component()
        } else {
            component
        };

        let port = parent
            .find_port(&self.0.port)
            .ok_or(linker::Error::undefined_port(parent.name(), &self.0.port))?;

        Ok(PortPins::new(port.key().0, self.0.range))
    }
}
