use std::fmt;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use ustr::{ustr, Ustr};

use super::{
    linker::{self, Error, KnownComponents, Resolve},
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
pub struct ComponentRefs {
    reference: ComponentRefId,
    range: ReferenceRange,
}

impl ComponentRefs {
    pub(crate) fn new(reference: ComponentRefId, range: ReferenceRange) -> Self {
        Self {
            reference: reference.id(),
            range,
        }
    }

    pub fn id(&self) -> ComponentRefId {
        self.reference
    }

    pub fn reference<'m>(&self, module: &'m Module) -> ComponentRef<'m> {
        self.bind(module)
    }

    pub fn range(&self, module: &Module) -> Range<u32> {
        self.range.expand(module.lookup(self.reference).n_instances)
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

    pub fn simplify(&mut self, module: &Module) {
        self.range
            .flatten(module.lookup(self.reference).n_instances);
    }

    pub fn to_weak(&self, module: &Module) -> WeakReferences {
        let mut range = self.range.clone();
        range.flatten(module.lookup(self.reference).n_instances);

        WeakReferences {
            reference: self.reference(module).component().data().name,
            range,
        }
    }

    pub fn print<'m>(&self, module: &'m Module, style: PrintStyle) -> PrintComponentRefs<'_, 'm> {
        PrintComponentRefs {
            module,
            reference: self,
            style,
        }
    }
}

impl ComponentRefAccess for &ComponentRefs {
    fn id(&self) -> ComponentRefId {
        self.reference
    }

    fn bind<'m>(&self, module: &'m Module) -> ComponentRef<'m> {
        self.reference.bind(module)
    }
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub enum PrintStyle {
    Python,
    #[default]
    Rust,
}

impl PrintStyle {
    pub fn range_sep(&self) -> &'static str {
        match self {
            Self::Python => ":",
            Self::Rust => "..",
        }
    }
}

pub struct PrintComponentRefs<'a, 'm> {
    module: &'m Module,
    reference: &'a ComponentRefs,
    style: PrintStyle,
}

impl<'a, 'm> fmt::Display for PrintComponentRefs<'a, 'm> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let component = self.reference.reference(self.module).component().name();

        let suffix = if self.reference.len(self.module) > 1 {
            let Range { start, end } = self
                .reference
                .range
                .expand(self.reference.reference(self.module).n_instances());

            let sep = self.style.range_sep();
            format!("[{start}{sep}{end}]")
        } else {
            String::new()
        };

        write!(f, "{component}{suffix}")
    }
}

mod connection_access {
    use super::*;

    pub trait Sealed {}

    impl Sealed for ConnectionId {}

    impl Sealed for Connection<'_> {}
}

pub trait ConnectionAccess: Copy + connection_access::Sealed {
    fn id(&self) -> ConnectionId;

    fn bind<'m>(&self, module: &'m Module) -> Connection<'m>;
}

impl ConnectionAccess for ConnectionId {
    fn id(&self) -> ConnectionId {
        *self
    }

    fn bind<'m>(&self, module: &'m Module) -> Connection<'m> {
        Connection::new(module, *self)
    }
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub(crate) struct ConnectionData {
    pub name: Option<Ustr>,
    pub kind: ConnectionKind,
    pub source_component: Option<ComponentRefs>,
    pub source_pins: PortPins,
    pub sink_component: Option<ComponentRefs>,
    pub sink_pins: PortPins,
}

impl ConnectionData {
    pub(crate) fn new(
        name: Option<Ustr>,
        kind: ConnectionKind,
        source_pins: PortPins,
        sink_pins: PortPins,
        source_component: Option<ComponentRefs>,
        sink_component: Option<ComponentRefs>,
    ) -> Self {
        Self {
            name,
            kind,
            source_component,
            source_pins,
            sink_component,
            sink_pins,
        }
    }
}

#[derive(Debug)]
pub enum ConnectionName {
    Named(String),
    Signature(Signature),
}

impl fmt::Display for ConnectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(name) => {
                write!(f, "{name}")
            }
            Self::Signature(signature) => {
                write!(f, "{signature}")
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Connection<'m>(&'m Module, ConnectionId);

impl<'m> Connection<'m> {
    fn new(module: &'m Module, connection: ConnectionId) -> Self {
        Self(module, connection)
    }

    pub fn unbind(self) -> ConnectionId {
        self.1
    }

    pub(crate) fn data(&self) -> &'m ConnectionData {
        self.0.lookup(self.1)
    }

    pub fn source_parent(&self) -> Component<'m> {
        if let Some(reference) = &self.data().source_component {
            reference.reference(self.0).parent()
        } else {
            self.source_pins().port(self.0).parent()
        }
    }

    pub fn source_name_or_default(&self) -> ConnectionName {
        if let Some(name) = self.data().name {
            ConnectionName::Named(name.to_string())
        } else {
            ConnectionName::Signature(Signature {
                parent: Some(self.source_parent().data().name.to_string()),
                reference: self
                    .source_component()
                    .map(|reference| reference.to_weak(self.0)),
                pins: self.source_pins().to_weak(self.0),
            })
        }
    }

    pub fn sink_parent(&self) -> Component<'m> {
        if let Some(reference) = &self.data().sink_component {
            reference.reference(self.0).parent()
        } else {
            self.sink_pins().port(self.0).parent()
        }
    }

    pub fn sink_name_or_default(&self) -> ConnectionName {
        if let Some(name) = self.data().name {
            ConnectionName::Named(name.to_string())
        } else {
            ConnectionName::Signature(Signature {
                parent: Some(self.sink_parent().data().name.to_string()),
                reference: self
                    .sink_component()
                    .map(|reference| reference.to_weak(self.0)),
                pins: self.sink_pins().to_weak(self.0),
            })
        }
    }

    pub fn source_component(&self) -> Option<&ComponentRefs> {
        self.0.lookup(self.1).source_component.as_ref()
    }

    pub fn source_pins(&self) -> &PortPins {
        &self.0.lookup(self.1).source_pins
    }

    pub fn sink_component(&self) -> Option<&ComponentRefs> {
        self.0.lookup(self.1).sink_component.as_ref()
    }

    pub fn sink_pins(&self) -> &PortPins {
        &self.0.lookup(self.1).sink_pins
    }

    pub fn kind(&self) -> ConnectionKind {
        self.0.lookup(self.1).kind
    }
}

impl ConnectionAccess for Connection<'_> {
    fn id(&self) -> ConnectionId {
        self.1
    }

    fn bind<'m>(&self, module: &'m Module) -> Connection<'m> {
        self.1.bind(module)
    }
}

pub struct SourceSet(PortPins, Option<ComponentRefs>);
pub struct SourceUnset;
pub struct SinkSet(PortPins, Option<ComponentRefs>);
pub struct SinkUnset;

pub struct ConnectionBuilder<'a, 'm, Src, Snk> {
    module: &'m mut Module,
    checker: &'a mut Checker,
    component: ComponentId,
    name: Option<Ustr>,
    source: Src,
    sink: Snk,
    kind: Option<ConnectionKind>,
}

pub type ConnectionBuilderNew<'a, 'm> = ConnectionBuilder<'a, 'm, SourceUnset, SinkUnset>;

impl<'a, 'm> ConnectionBuilder<'a, 'm, SourceUnset, SinkUnset> {
    pub(super) fn new<C: ComponentAccess>(
        module: &'m mut Module,
        checker: &'a mut Checker,
        component: C,
    ) -> Self {
        Self {
            module,
            checker,
            component: component.id(),
            name: None,
            source: SourceUnset,
            sink: SinkUnset,
            kind: None,
        }
    }
}

impl<'a, 'm, Snk> ConnectionBuilder<'a, 'm, SourceUnset, Snk> {
    pub fn set_source(
        self,
        mut pins: PortPins,
        mut component: Option<ComponentRefs>,
    ) -> ConnectionBuilder<'a, 'm, SourceSet, Snk> {
        pins.simplify(self.module);

        if let Some(component) = component.as_mut() {
            component.simplify(self.module)
        }

        ConnectionBuilder {
            module: self.module,
            checker: self.checker,
            component: self.component,
            name: self.name,
            source: SourceSet(pins, component),
            sink: self.sink,
            kind: self.kind,
        }
    }
}

impl<'a, 'm, Src> ConnectionBuilder<'a, 'm, Src, SinkUnset> {
    pub fn set_sink(
        self,
        mut pins: PortPins,
        mut component: Option<ComponentRefs>,
    ) -> ConnectionBuilder<'a, 'm, Src, SinkSet> {
        pins.simplify(self.module);

        if let Some(component) = component.as_mut() {
            component.simplify(self.module)
        }

        ConnectionBuilder {
            module: self.module,
            checker: self.checker,
            component: self.component,
            name: self.name,
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

    pub fn set_name(&mut self, name: &str) {
        self.name = Some(ustr(name));
    }

    pub fn name_is_set(&self) -> bool {
        self.name.is_some()
    }
}

impl<'a, 'm> ConnectionBuilder<'a, 'm, SourceSet, SinkSet> {
    fn insert(self) -> (&'m mut Module, &'a mut Checker, ComponentId, ConnectionId) {
        let kind = self.kind.unwrap_or(ConnectionKind::Direct);

        let SourceSet(source_pins, source_component) = self.source;
        let SinkSet(sink_pins, sink_component) = self.sink;

        let connection = self.module.connections.insert(ConnectionData::new(
            self.name,
            kind,
            source_pins,
            sink_pins,
            source_component,
            sink_component,
        ));

        (self.module, self.checker, self.component, connection)
    }

    pub fn finish(self) -> linker::Result<Connection<'m>> {
        let (module, checker, parent, connection) = self.insert();
        checker.register_connection(module, connection)?;
        module.lookup_mut(parent).connections.push(connection);
        Ok(connection.bind(module))
    }
}

#[derive(Clone, Debug)]
enum ComponentOrRefs {
    Component(ComponentId),
    Reference(ComponentRefs),
}

impl ComponentOrRefs {
    pub fn get_reference(&self) -> Option<&ComponentRefs> {
        match self {
            Self::Component(_) => None,
            Self::Reference(reference) => Some(reference),
        }
    }

    pub fn into_reference(self) -> Option<ComponentRefs> {
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

    pub fn try_into_masked(self, start: Option<u32>, end: Option<u32>) -> Option<Self> {
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

#[derive(Debug)]
struct ConcatSource {
    component: ComponentOrRefs,
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
        self.sink_start_i(sink_n_pins) < self.sink_last_i(sink_n_pins)
    }
}

#[derive(Debug)]
pub struct Concat {
    sink_component: ComponentOrRefs,
    sink_port: PortPins,
    pin_index: u32,
    sources: Vec<ConcatSource>,
}

struct Splitter<'a, 'm> {
    module: &'m mut Module,
    checker: &'a mut Checker,
    parent: ComponentId,
    source: ConcatSource,
    sink_n_pins: u32,
    sink_start_i: u32,
    sink_last_i: u32,
}

impl Concat {
    fn new(sink_component: ComponentOrRefs, sink_port: PortPins) -> Self {
        Self {
            sink_component,
            sink_port,
            pin_index: 0,
            sources: Vec::new(),
        }
    }

    pub fn new_component(sink_component: ComponentId, sink_port: PortPins) -> Self {
        let sink_component = ComponentOrRefs::Component(sink_component);
        Self::new(sink_component, sink_port)
    }

    pub fn new_reference(sink_component: ComponentRefs, sink_port: PortPins) -> Self {
        let sink_component = ComponentOrRefs::Reference(sink_component);
        Self::new(sink_component, sink_port)
    }

    fn append_source(
        &mut self,
        module: &Module,
        source_component: ComponentOrRefs,
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

    pub fn append_component(
        &mut self,
        module: &Module,
        source_component: ComponentId,
        source_pins: PortPins,
    ) {
        let source_component = ComponentOrRefs::Component(source_component);
        self.append_source(module, source_component, source_pins)
    }

    pub fn append_reference(
        &mut self,
        module: &Module,
        source_reference: ComponentRefs,
        source_pins: PortPins,
    ) {
        let source_reference = ComponentOrRefs::Reference(source_reference);
        self.append_source(module, source_reference, source_pins)
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

    fn split_and_connect(self, splitter: Splitter) -> linker::Result<()> {
        use std::iter::zip;

        let Splitter {
            module,
            checker,
            parent,
            source,
            sink_n_pins,
            sink_start_i,
            sink_last_i,
        } = splitter;

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
            let source_pins = source.pins.clone_masked(part.source_pins);

            let source_component = source
                .component
                .clone()
                .try_into_masked(Some(source_port_i), Some(source_port_i + 1))
                .expect("should be a valid mask");

            let sink_pins = self.sink_port.clone_masked(part.sink_pins);

            let sink_component = self
                .sink_component
                .clone()
                .try_into_masked(Some(part.sink_port_i), Some(part.sink_port_i + 1))
                .expect("should be a valid mask");

            ConnectionBuilder::new(module, checker, parent)
                .set_source(source_pins, source_component.into_reference())
                .set_sink(sink_pins, sink_component.into_reference())
                .finish()?;
        }

        Ok(())
    }

    pub fn make_connections(
        self,
        module: &mut Module,
        checker: &mut Checker,
    ) -> linker::Result<()> {
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
                    .finish()?;
            } else {
                let splitter = Self::new(self.sink_component.clone(), self.sink_port.clone());

                splitter.split_and_connect(Splitter {
                    module,
                    checker,
                    parent,
                    source,
                    sink_n_pins,
                    sink_start_i,
                    sink_last_i,
                })?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct WeakReferences {
    reference: Ustr,
    #[serde(flatten)]
    range: ReferenceRange,
}

impl WeakReferences {
    pub fn new(reference: &str, start: Option<u32>, end: Option<u32>) -> Self {
        Self {
            reference: ustr(reference),
            range: ReferenceRange::new(start, end),
        }
    }

    pub fn to_string(&self, style: PrintStyle) -> String {
        let reference = &self.reference;

        let suffix = match (self.range.get_start(), self.range.get_end()) {
            (None, None) => "".to_string(),
            (start, end) => {
                let start = start
                    .map(|start| start.to_string())
                    .unwrap_or("".to_string());

                let end = end.map(|end| end.to_string()).unwrap_or("".to_string());
                let sep = style.range_sep();
                format!("{start}{sep}{end}")
            }
        };

        format!("{reference}{suffix}")
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct Signature {
    #[serde(skip)]
    pub parent: Option<String>,
    #[serde(flatten)]
    pub pins: WeakPortPins,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub reference: Option<WeakReferences>,
}

impl Signature {
    pub fn new(
        module: &Module,
        source_pins: &PortPins,
        source_reference: Option<&ComponentRefs>,
    ) -> Self {
        let pins = source_pins.to_weak(module);

        if let Some(reference) = source_reference {
            let parent = reference.reference(module).parent().data().name;
            let reference = reference.to_weak(module);

            Self {
                parent: Some(parent.to_string()),
                reference: Some(reference),
                pins,
            }
        } else {
            let parent = source_pins.port(module).parent().data().name;

            Self {
                parent: Some(parent.to_string()),
                reference: None,
                pins,
            }
        }
    }

    pub fn to_string(&self, style: PrintStyle) -> String {
        let reference = self
            .reference
            .as_ref()
            .map(|reference| format!(".{}", reference.to_string(style)))
            .unwrap_or("".to_string());

        let pins = format!(".{}", self.pins.to_string(style));
        let root = self.parent.as_deref().unwrap_or("");
        format!("{root}{reference}{pins}")
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string(PrintStyle::default()))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct WeakConnection {
    pub kind: ConnectionKind,
    pub source: Signature,
    pub sink: Signature,
}

pub struct WeakSourceSet(WeakPortPins, Option<WeakReferences>);
pub struct WeakSourceUnset;
pub struct WeakSinkSet(WeakPortPins, Option<WeakReferences>);
pub struct WeakSinkUnset;

#[derive(Default)]
pub struct WeakConnectionBuilder<Src, Snk> {
    parent: Option<String>,
    source: Src,
    sink: Snk,
    kind: Option<ConnectionKind>,
}

impl WeakConnectionBuilder<WeakSourceUnset, WeakSinkUnset> {
    pub fn new() -> Self {
        Self {
            parent: None,
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
            parent: self.parent,
            source: WeakSourceSet(
                pins,
                component.map(|component| {
                    WeakReferences::new(component, reference_start, reference_end)
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
            parent: self.parent,
            source: self.source,
            sink: WeakSinkSet(
                pins,
                component.map(|component| {
                    WeakReferences::new(component, reference_start, reference_end)
                }),
            ),
            kind: self.kind,
        }
    }
}

impl<Src, Snk> WeakConnectionBuilder<Src, Snk> {
    pub fn set_parent(&mut self, parent: String) {
        self.parent = Some(parent);
    }

    pub fn parent_is_set(&self) -> bool {
        self.parent.is_some()
    }

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
                parent: self.parent.clone(),
                pins: self.source.0,
                reference: self.source.1,
            },
            sink: Signature {
                parent: self.parent,
                pins: self.sink.0,
                reference: self.sink.1,
            },
            kind: self.kind.unwrap_or_default(),
        }
    }
}

impl<'a, 'm> Resolve<'a, 'm> for Signature {
    type Output = (PortPins, Option<ComponentRefs>);

    fn resolve<C: ComponentAccess>(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
        parent: C,
        components: &KnownComponents,
    ) -> linker::Result<Self::Output> {
        let component = parent.bind(module);

        let reference = self
            .reference
            .map(|ref reference| {
                let start = reference.range.get_start();
                let end = reference.range.get_end();
                component
                    .find_reference(&reference.reference)
                    .ok_or(Error::undefined_reference(
                        component.name(),
                        &reference.reference,
                    ))
                    .map(|reference| {
                        ComponentRefs::new(reference.id(), ReferenceRange::new(start, end))
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
    type Output = Connection<'m>;

    fn resolve<C: ComponentAccess>(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
        parent: C,
        components: &KnownComponents,
    ) -> linker::Result<Self::Output> {
        let (source_pins, source_reference) =
            self.source.resolve(module, checker, parent, components)?;

        let (sink_pins, sink_reference) = self.sink.resolve(module, checker, parent, components)?;

        let mut builder = ConnectionBuilder::new(module, checker, parent)
            .set_source(source_pins, source_reference)
            .set_sink(sink_pins, sink_reference);

        builder.set_kind(self.kind);
        builder.finish()
    }
}
