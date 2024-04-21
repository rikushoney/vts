use std::ops::Range;

use serde::{Deserialize, Serialize};
use ustr::{ustr, Ustr};

use super::{
    checker,
    linker::{self, KnownComponents, Resolve},
    prelude::*,
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
    pub name: Ustr,
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
            name: ustr(name),
            parent,
            kind,
            n_pins,
            class,
        }
    }
}

mod port_access {
    use super::*;

    pub trait Sealed {}

    impl Sealed for PortId {}

    impl Sealed for Port<'_> {}
}

pub trait PortAccess: port_access::Sealed {
    fn id(&self) -> PortId;
    fn bind<'m>(&self, module: &'m Module) -> Port<'m>;
}

impl PortAccess for PortId {
    fn id(&self) -> PortId {
        *self
    }

    fn bind<'m>(&self, module: &'m Module) -> Port<'m> {
        Port::new(module, self.id())
    }
}

#[derive(Clone, Debug)]
pub struct Port<'m>(&'m Module, PortId);

impl<'m> Port<'m> {
    fn new(module: &'m Module, port: PortId) -> Self {
        Self(module, port)
    }

    pub fn module(&self) -> &'m Module {
        self.0
    }

    pub fn unbind(self) -> PortId {
        self.1
    }

    pub fn parent(&self) -> Component<'_> {
        self.data().parent.bind(self.0)
    }

    pub fn name(&self) -> &str {
        &self.module().lookup(self.1).name
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

    pub fn select_all(&self) -> PortPins {
        self.select(PinRange::Full)
    }

    pub fn select_range(&self, start: Option<u32>, end: Option<u32>) -> PortPins {
        self.select(PinRange::new(start, end))
    }
}

impl PortAccess for Port<'_> {
    fn id(&self) -> PortId {
        self.1
    }

    fn bind<'m>(&self, module: &'m Module) -> Port<'m> {
        self.1.bind(module)
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum PinRange {
    Start(u32),
    End(u32),
    Bound(Range<u32>),
    #[default]
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

    #[must_use]
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
                    *self = Self::Full;
                }
            }
            Self::End(end) => {
                if *end == n_pins {
                    *self = Self::Full;
                }
            }
            Self::Bound(range) => {
                if range.start == 0 {
                    *self = Self::End(range.end);
                    return self.flatten(n_pins);
                }

                if range.end == n_pins {
                    *self = Self::Start(range.start);
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

    pub fn len<'m>(&self, module: &'m Module) -> u32 {
        let n_pins = self.port(module).n_pins();
        let Range { start, end } = self.range.expand(n_pins);
        debug_assert!(end >= start);
        end - start
    }

    pub fn mask(&mut self, start: Option<u32>, end: Option<u32>) {
        self.range = match self.range {
            PinRange::Start(start) => PinRange::new(Some(start), end),
            PinRange::End(end) => PinRange::new(start, Some(end)),
            PinRange::Bound(ref range) => {
                let mut range = range.clone();

                if let Some(start) = start {
                    range.start = start;
                }

                if let Some(end) = end {
                    range.end = end;
                }

                PinRange::Bound(range)
            }
            PinRange::Full => PinRange::new(start, end),
        };
    }
}

pub struct NameSet(Ustr);
pub struct NameUnset;
pub struct KindSet(PortKind);
pub struct KindUnset;

pub struct PortBuilder<'a, 'm, N, K> {
    module: &'m mut Module,
    checker: &'a mut Checker,
    parent: ComponentId,
    name: N,
    kind: K,
    n_pins: Option<u32>,
    class: Option<PortClass>,
}

impl<'a, 'm> PortBuilder<'a, 'm, NameUnset, KindUnset> {
    pub fn new<C: ComponentAccess>(
        module: &'m mut Module,
        checker: &'a mut Checker,
        component: C,
    ) -> Self {
        Self {
            module,
            checker,
            parent: component.id(),
            name: NameUnset,
            kind: KindUnset,
            n_pins: None,
            class: None,
        }
    }
}

impl<'a, 'm, K> PortBuilder<'a, 'm, NameUnset, K> {
    pub fn set_name(self, name: &str) -> PortBuilder<'a, 'm, NameSet, K> {
        PortBuilder {
            module: self.module,
            checker: self.checker,
            parent: self.parent,
            name: NameSet(ustr(name)),
            kind: self.kind,
            n_pins: self.n_pins,
            class: self.class,
        }
    }
}

impl<'a, 'm, N> PortBuilder<'a, 'm, N, KindUnset> {
    pub fn set_kind(self, kind: PortKind) -> PortBuilder<'a, 'm, N, KindSet> {
        PortBuilder {
            module: self.module,
            checker: self.checker,
            parent: self.parent,
            name: self.name,
            kind: KindSet(kind),
            n_pins: self.n_pins,
            class: self.class,
        }
    }
}

impl<'a, 'm, N, K> PortBuilder<'a, 'm, N, K> {
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

impl<'a, 'm> PortBuilder<'a, 'm, NameSet, KindSet> {
    fn insert(&mut self) -> PortId {
        let port = PortData::new(
            self.parent.id(),
            &self.name.0,
            self.kind.0,
            self.n_pins.unwrap_or(1),
            self.class,
        );

        self.module.ports.insert(port)
    }

    pub fn finish(mut self) -> Result<Port<'m>, checker::Error> {
        let port = self.insert();
        self.checker.register_port(self.module, self.parent, port)?;
        self.module.lookup_mut(self.parent).ports.push(port);
        Ok(Port::new(self.module, port))
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct WeakPortPins {
    pub port: Ustr,
    #[serde(flatten)]
    pub range: PinRange,
}

impl<'a, 'm> Resolve<'a, 'm> for (WeakPortPins, Option<ComponentRefId>) {
    type Output = PortPins;

    fn resolve<C: ComponentAccess>(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
        parent: C,
        _components: &KnownComponents,
    ) -> Result<Self::Output, linker::Error> {
        let component = parent.bind(module);

        let parent = if let Some(reference) = self.1 {
            reference.bind(module).component()
        } else {
            component
        };

        let port = parent
            .find_port(&self.0.port)
            .ok_or(linker::Error::undefined_port(parent.name(), &self.0.port))?
            .unbind();

        checker.register_connection()?;
        Ok(PortPins::new(port, self.0.range))
    }
}
