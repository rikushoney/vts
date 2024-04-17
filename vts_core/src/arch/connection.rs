use serde::{Deserialize, Serialize};
use ustr::{ustr, Ustr};

use super::{
    component::ComponentKey,
    linker::{self, KnownComponents, Resolve},
    port::{PortPins, WeakPortPins},
    prelude::*,
    reference::{ComponentRefKey, ReferenceRange},
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
pub struct ComponentRefSelection(pub(super) ComponentRefId, pub(super) ReferenceRange);

impl ComponentRefSelection {
    pub fn new(reference: ComponentRefKey, range: ReferenceRange) -> Self {
        Self(reference.0, range)
    }

    pub fn key(&self) -> ComponentRefKey {
        ComponentRefKey::new(self.0)
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
    pub fn new(module: &'m mut Module, checker: &'a mut Checker, component: ComponentKey) -> Self {
        Self {
            module,
            checker,
            component: component.0,
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

        let connections = &mut self.module[self.component].connections;
        let idx = connections.len();
        connections.push(connection);
        &connections[idx]
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

    fn resolve(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
        parent: ComponentKey,
        components: &KnownComponents,
    ) -> Result<Self::Output, linker::Error> {
        let component = Component::new(module, parent.0);

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
                        ComponentRefSelection::new(reference.key(), ReferenceRange::new(start, end))
                    })
            })
            .transpose()?;

        let resolver = (
            self.pins,
            reference
                .as_ref()
                .map(|reference| ComponentRefKey::new(reference.0)),
        );

        let pins = resolver.resolve(module, checker, parent, components)?;

        Ok((pins, reference))
    }
}

impl<'a, 'm> Resolve<'a, 'm> for WeakConnection {
    type Output = &'m Connection;

    fn resolve(
        self,
        module: &'m mut Module,
        checker: &'a mut Checker,
        parent: ComponentKey,
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
