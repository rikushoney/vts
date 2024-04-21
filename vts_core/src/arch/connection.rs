use std::ops::Range;

use serde::{Deserialize, Serialize};
use ustr::{ustr, Ustr};

use super::{
    linker::{self, KnownComponents, Resolve},
    module::ComponentRefId,
    port::{PortPins, WeakPortPins},
    prelude::*,
    reference::{ComponentRefAccess, ReferenceRange},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionKind {
    #[default]
    Direct,
    Complete,
    Mux,
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ComponentRefSelection {
    reference: ComponentRefId,
    range: ReferenceRange,
}

impl ComponentRefSelection {
    pub fn new(reference: ComponentRefId, range: ReferenceRange) -> Self {
        Self {
            reference: reference.id(),
            range,
        }
    }

    pub fn id(&self) -> ComponentRefId {
        self.reference
    }

    pub fn reference<'m>(&self, module: &'m Module) -> ComponentRef<'m> {
        ComponentRef::new(module, self.reference)
    }

    pub fn get_start(&self) -> Option<u32> {
        self.range.get_start()
    }

    pub fn get_end(&self) -> Option<u32> {
        self.range.get_end()
    }

    pub fn len(&self, module: &Module) -> u32 {
        module.lookup(self.reference).n_instances
    }

    pub fn mask(&mut self, start: Option<u32>, end: Option<u32>) {
        self.range = match self.range {
            ReferenceRange::Start(start) => ReferenceRange::new(Some(start), end),
            ReferenceRange::End(end) => ReferenceRange::new(start, Some(end)),
            ReferenceRange::Bound(ref range) => {
                let mut range = range.clone();

                if let Some(start) = start {
                    range.start = start;
                }

                if let Some(end) = end {
                    range.end = end;
                }

                ReferenceRange::Bound(range)
            }
            ReferenceRange::Full => ReferenceRange::new(start, end),
        };
    }
}

impl ComponentRefAccess for ComponentRefSelection {
    fn id(&self) -> ComponentRefId {
        self.reference
    }

    fn bind<'m>(&self, module: &'m Module) -> ComponentRef<'m> {
        self.reference.bind(module)
    }
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Connection {
    pub kind: ConnectionKind,
    pub(crate) source_component: Option<ComponentRefSelection>,
    pub(crate) source_pins: PortPins,
    pub(crate) sink_component: Option<ComponentRefSelection>,
    pub(crate) sink_pins: PortPins,
}

impl Connection {
    pub(crate) fn new(
        kind: ConnectionKind,
        source_pins: PortPins,
        sink_pins: PortPins,
        source_component: Option<ComponentRefSelection>,
        sink_component: Option<ComponentRefSelection>,
    ) -> Self {
        Self {
            kind,
            source_component,
            source_pins,
            sink_component,
            sink_pins,
        }
    }

    pub fn source_pins(&self) -> &PortPins {
        &self.source_pins
    }

    pub fn sink_pins(&self) -> &PortPins {
        &self.sink_pins
    }
}

pub struct SourceSet(PortPins, Option<ComponentRefSelection>);
pub struct SourceUnset;
pub struct SinkSet(PortPins, Option<ComponentRefSelection>);
pub struct SinkUnset;

pub struct ConnectionBuilder<'a, 'm, Src, Snk> {
    module: &'m mut Module,
    checker: &'a mut Checker,
    component: ComponentId,
    source: Src,
    sink: Snk,
    kind: Option<ConnectionKind>,
}

impl<'a, 'm> ConnectionBuilder<'a, 'm, SourceUnset, SinkUnset> {
    pub fn new<C: ComponentAccess>(
        module: &'m mut Module,
        checker: &'a mut Checker,
        component: C,
    ) -> Self {
        Self {
            module,
            checker,
            component: component.id(),
            source: SourceUnset,
            sink: SinkUnset,
            kind: None,
        }
    }
}

impl<'a, 'm, Snk> ConnectionBuilder<'a, 'm, SourceUnset, Snk> {
    pub fn set_source(
        self,
        pins: PortPins,
        component: Option<ComponentRefSelection>,
    ) -> ConnectionBuilder<'a, 'm, SourceSet, Snk> {
        ConnectionBuilder {
            module: self.module,
            checker: self.checker,
            component: self.component,
            source: SourceSet(pins, component),
            sink: self.sink,
            kind: self.kind,
        }
    }
}

impl<'a, 'm, Src> ConnectionBuilder<'a, 'm, Src, SinkUnset> {
    pub fn set_sink(
        self,
        pins: PortPins,
        component: Option<ComponentRefSelection>,
    ) -> ConnectionBuilder<'a, 'm, Src, SinkSet> {
        ConnectionBuilder {
            module: self.module,
            checker: self.checker,
            component: self.component,
            source: self.source,
            sink: SinkSet(pins, component),
            kind: self.kind,
        }
    }
}

impl<'a, 'm, Src, Snk> ConnectionBuilder<'a, 'm, Src, Snk> {
    pub fn set_kind(&mut self, kind: ConnectionKind) {
        self.kind = Some(kind);
    }

    pub fn kind_is_set(&self) -> bool {
        self.kind.is_some()
    }
}

impl<'a, 'm> ConnectionBuilder<'a, 'm, SourceSet, SinkSet> {
    pub fn finish(self) -> &'m Connection {
        let kind = self.kind.unwrap_or(ConnectionKind::Direct);

        let connection =
            Connection::new(kind, self.source.0, self.sink.0, self.source.1, self.sink.1);

        let connections = &mut self.module.lookup_mut(self.component).connections;
        let idx = connections.len();
        connections.push(connection);
        &connections[idx]
    }
}

#[derive(Clone, Debug)]
enum ComponentOrRefSelection {
    Component(ComponentId),
    Reference(ComponentRefSelection),
}

impl ComponentOrRefSelection {
    pub fn get_reference(&self) -> Option<&ComponentRefSelection> {
        match self {
            Self::Component(_) => None,
            Self::Reference(reference) => Some(reference),
        }
    }

    pub fn into_reference(self) -> Option<ComponentRefSelection> {
        match self {
            Self::Component(_) => None,
            Self::Reference(reference) => Some(reference),
        }
    }

    pub fn len(&self, module: &Module) -> u32 {
        match self {
            Self::Component(_) => 1,
            Self::Reference(reference) => reference.len(module),
        }
    }

    pub fn parent<'m>(&self, module: &'m Module) -> Component<'m> {
        match self {
            Self::Component(component) => component.bind(module),
            Self::Reference(reference) => reference.reference(module).parent(),
        }
    }

    pub fn mask(self, start: Option<u32>, end: Option<u32>) -> Option<Self> {
        match self {
            Self::Component(component) => match (start, end) {
                (Some(0), Some(1)) | (Some(0), None) | (None, Some(1)) | (None, None) => {
                    Some(Self::Component(component))
                }
                _ => None,
            },
            Self::Reference(mut reference) => {
                reference.mask(start, end);
                Some(Self::Reference(reference))
            }
        }
    }
}

struct SourceToSink {
    source_pins: Range<u32>,
    sink_port_i: u32,
    sink_pins: Range<u32>,
}

struct ConcatSource {
    component: ComponentOrRefSelection,
    pins: PortPins,
    sink_pins: Range<u32>,
}

impl ConcatSource {
    pub fn sink_start_i(&self, sink_n_pins: u32) -> u32 {
        self.sink_pins.start / sink_n_pins
    }

    pub fn sink_last_i(&self, sink_n_pins: u32) -> u32 {
        (self.sink_pins.end - 1) / sink_n_pins
    }

    pub fn needs_multiple_connections(&self, sink_n_pins: u32) -> bool {
        self.sink_start_i(sink_n_pins) == self.sink_last_i(sink_n_pins)
    }
}

pub struct Concat {
    sink_component: ComponentOrRefSelection,
    sink_port: PortPins,
    pin_index: u32,
    sources: Vec<ConcatSource>,
}

impl Concat {
    fn new(sink_component: ComponentOrRefSelection, sink_port: PortPins) -> Self {
        Self {
            sink_component,
            sink_port,
            pin_index: 0,
            sources: Vec::new(),
        }
    }

    pub fn new_component(sink_component: ComponentId, sink_port: PortPins) -> Self {
        let sink_component = ComponentOrRefSelection::Component(sink_component);
        Self::new(sink_component, sink_port)
    }

    pub fn new_reference(sink_component: ComponentRefSelection, sink_port: PortPins) -> Self {
        let sink_component = ComponentOrRefSelection::Reference(sink_component);
        Self::new(sink_component, sink_port)
    }

    fn append_source(
        &mut self,
        module: &Module,
        source_component: ComponentOrRefSelection,
        source_pins: PortPins,
    ) {
        let source_pin_count = source_component.len(module) * source_pins.len(module);
        let sink_start = self.pin_index;
        let sink_end = sink_start + source_pin_count;
        self.pin_index = sink_end;

        self.sources.push(ConcatSource {
            component: source_component,
            pins: source_pins,
            sink_pins: Range {
                start: sink_start,
                end: sink_end,
            },
        })
    }

    pub fn append_component_source(
        &mut self,
        module: &Module,
        source_component: ComponentId,
        source_pins: PortPins,
    ) {
        let source_component = ComponentOrRefSelection::Component(source_component);
        self.append_source(module, source_component, source_pins)
    }

    pub fn append_reference_source(
        &mut self,
        module: &Module,
        source_component: ComponentRefSelection,
        source_pins: PortPins,
    ) {
        let source_component = ComponentOrRefSelection::Reference(source_component);
        self.append_source(module, source_component, source_pins)
    }

    // TODO: make lazy instead
    fn partition_connections(
        module: &Module,
        source: &ConcatSource,
        source_n_pins: u32,
        sink_n_pins: u32,
        sink_start_i: u32,
        sink_last_i: u32,
    ) -> Vec<SourceToSink> {
        let mut splits = Vec::new();
        let mut pin_budget = (sink_start_i + 1) * sink_n_pins - source.sink_pins.start;
        debug_assert!(pin_budget > 0);
        let mut sink_port_i = sink_start_i;

        for _ in 0..source.component.len(module) {
            let mut pins_left = source_n_pins;
            let mut source_pin_i = 0;

            while pin_budget < pins_left {
                splits.push(SourceToSink {
                    source_pins: Range {
                        start: source_pin_i,
                        end: source_pin_i + pin_budget,
                    },
                    sink_port_i,
                    sink_pins: Range {
                        start: sink_n_pins - pin_budget,
                        end: sink_n_pins,
                    },
                });

                source_pin_i += pin_budget;
                pins_left -= pin_budget;
                sink_port_i += 1;
                pin_budget = sink_n_pins;
            }

            debug_assert_eq!(sink_port_i, sink_last_i);

            splits.push(SourceToSink {
                source_pins: Range {
                    start: source_pin_i,
                    end: source_n_pins,
                },
                sink_port_i,
                sink_pins: Range {
                    start: 0,
                    end: pins_left,
                },
            });
        }

        splits
    }

    fn split_and_connect(
        self,
        module: &mut Module,
        checker: &mut Checker,
        parent: ComponentId,
        source: ConcatSource,
        sink_n_pins: u32,
        sink_start_i: u32,
        sink_last_i: u32,
    ) {
        use std::iter::zip;

        debug_assert!(sink_start_i < sink_last_i);
        let source_n_pins = source.pins.len(module);

        let splits = Self::partition_connections(
            module,
            &source,
            source_n_pins,
            sink_n_pins,
            sink_start_i,
            sink_last_i,
        );

        for (source_port_i, part) in zip(0.., splits.into_iter()) {
            let source_pins = {
                let mut pins = source.pins.clone();
                pins.mask(Some(part.source_pins.start), Some(part.source_pins.end));
                pins
            };

            let source_component = source
                .component
                .clone()
                .mask(Some(source_port_i), Some(source_port_i + 1))
                .expect("should be a valid mask");

            let sink_pins = {
                let mut pins = self.sink_port.clone();
                pins.mask(Some(part.sink_pins.start), Some(part.sink_pins.end));
                pins
            };

            let sink_component = self
                .sink_component
                .clone()
                .mask(Some(part.sink_port_i), Some(part.sink_port_i + 1))
                .expect("should be a valid mask");

            ConnectionBuilder::new(module, checker, parent)
                .set_source(source_pins, source_component.into_reference())
                .set_sink(sink_pins, sink_component.into_reference())
                .finish();
        }
    }

    pub fn make_connections(self, module: &mut Module, checker: &mut Checker) {
        let parent = self.sink_component.parent(module).id();
        let sink_n_pins = self.sink_port.len(module);

        for source in self.sources.into_iter() {
            let sink_start_i = source.sink_start_i(sink_n_pins);
            let sink_last_i = source.sink_last_i(sink_n_pins);

            if !source.needs_multiple_connections(sink_n_pins) {
                let pin_start_offset = sink_start_i * sink_n_pins;
                let pin_start = source.sink_pins.start - pin_start_offset;
                let pin_end = source.sink_pins.end - pin_start_offset;

                let sink_pins = self
                    .sink_port
                    .port(module)
                    .select_range(Some(pin_start), Some(pin_end));

                ConnectionBuilder::new(module, checker, parent)
                    .set_source(source.pins, source.component.into_reference())
                    .set_sink(sink_pins, self.sink_component.get_reference().cloned())
                    .finish();
            } else {
                let splitter = Self::new(self.sink_component.clone(), self.sink_port.clone());

                splitter.split_and_connect(
                    module,
                    checker,
                    parent,
                    source,
                    sink_n_pins,
                    sink_start_i,
                    sink_last_i,
                )
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct WeakReferenceSelection {
    reference: Ustr,
    #[serde(flatten)]
    range: ReferenceRange,
}

impl WeakReferenceSelection {
    pub fn new(reference: &str, start: Option<u32>, end: Option<u32>) -> Self {
        Self {
            reference: ustr(reference),
            range: ReferenceRange::new(start, end),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct Signature {
    #[serde(flatten)]
    pub pins: WeakPortPins,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub reference: Option<WeakReferenceSelection>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct WeakConnection {
    pub kind: ConnectionKind,
    pub source: Signature,
    pub sink: Signature,
}

pub struct WeakSourceSet(WeakPortPins, Option<WeakReferenceSelection>);
pub struct WeakSourceUnset;
pub struct WeakSinkSet(WeakPortPins, Option<WeakReferenceSelection>);
pub struct WeakSinkUnset;

#[derive(Default)]
pub struct WeakConnectionBuilder<Src, Snk> {
    source: Src,
    sink: Snk,
    kind: Option<ConnectionKind>,
}

impl WeakConnectionBuilder<WeakSourceUnset, WeakSinkUnset> {
    pub fn new() -> Self {
        Self {
            source: WeakSourceUnset,
            sink: WeakSinkUnset,
            kind: None,
        }
    }
}

impl<Snk> WeakConnectionBuilder<WeakSourceUnset, Snk> {
    pub fn set_source(
        self,
        pins: WeakPortPins,
        component: Option<&str>,
        reference_start: Option<u32>,
        reference_end: Option<u32>,
    ) -> WeakConnectionBuilder<WeakSourceSet, Snk> {
        WeakConnectionBuilder {
            source: WeakSourceSet(
                pins,
                component.map(|component| {
                    WeakReferenceSelection::new(component, reference_start, reference_end)
                }),
            ),
            sink: self.sink,
            kind: self.kind,
        }
    }
}

impl<Src> WeakConnectionBuilder<Src, WeakSinkUnset> {
    pub fn set_sink(
        self,
        pins: WeakPortPins,
        component: Option<&str>,
        reference_start: Option<u32>,
        reference_end: Option<u32>,
    ) -> WeakConnectionBuilder<Src, WeakSinkSet> {
        WeakConnectionBuilder {
            source: self.source,
            sink: WeakSinkSet(
                pins,
                component.map(|component| {
                    WeakReferenceSelection::new(component, reference_start, reference_end)
                }),
            ),
            kind: self.kind,
        }
    }
}

impl<Src, Snk> WeakConnectionBuilder<Src, Snk> {
    pub fn set_kind(&mut self, kind: ConnectionKind) {
        self.kind = Some(kind);
    }

    pub fn kind_is_set(&self) -> bool {
        self.kind.is_some()
    }
}

impl WeakConnectionBuilder<WeakSourceSet, WeakSinkSet> {
    pub fn finish(self) -> WeakConnection {
        WeakConnection {
            source: Signature {
                pins: self.source.0,
                reference: self.source.1,
            },
            sink: Signature {
                pins: self.sink.0,
                reference: self.sink.1,
            },
            kind: self.kind.unwrap_or_default(),
        }
    }
}

impl<'a, 'm> Resolve<'a, 'm> for Signature {
    type Output = (PortPins, Option<ComponentRefSelection>);

    fn resolve<C: ComponentAccess>(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
        parent: C,
        components: &KnownComponents,
    ) -> Result<Self::Output, linker::Error> {
        let component = parent.bind(module);

        let reference = self
            .reference
            .map(|ref reference| {
                let start = reference.range.get_start();
                let end = reference.range.get_end();
                component
                    .find_reference(&reference.reference)
                    .ok_or(linker::Error::undefined_reference(
                        component.name(),
                        &reference.reference,
                    ))
                    .map(|reference| {
                        ComponentRefSelection::new(reference.id(), ReferenceRange::new(start, end))
                    })
            })
            .transpose()?;

        let resolver = (
            self.pins,
            reference.as_ref().map(|reference| reference.reference.id()),
        );

        let pins = resolver.resolve(module, checker, parent, components)?;
        Ok((pins, reference))
    }
}

impl<'a, 'm> Resolve<'a, 'm> for WeakConnection {
    type Output = &'m Connection;

    fn resolve<C: ComponentAccess>(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
        parent: C,
        components: &KnownComponents,
    ) -> Result<Self::Output, linker::Error> {
        let (source_pins, source_reference) =
            self.source.resolve(module, checker, parent, components)?;

        let (sink_pins, sink_reference) = self.sink.resolve(module, checker, parent, components)?;

        let mut builder = ConnectionBuilder::new(module, checker, parent)
            .set_source(source_pins, source_reference)
            .set_sink(sink_pins, sink_reference);

        builder.set_kind(self.kind);

        Ok(builder.finish())
    }
}
