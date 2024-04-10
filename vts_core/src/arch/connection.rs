use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::component::ComponentKey;
use super::module::{ComponentId, ComponentRefId, Module};
use super::port::{PortPins, WeakPortPins};
use super::reference::ComponentRefKey;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConnectionKind {
    #[default]
    Direct,
    Complete,
    Mux,
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Connection {
    pub kind: ConnectionKind,
    pub(crate) source_component: Option<ComponentRefId>,
    pub(crate) source_pins: PortPins,
    pub(crate) sink_component: Option<ComponentRefId>,
    pub(crate) sink_pins: PortPins,
}

impl Connection {
    pub(crate) fn new(
        kind: ConnectionKind,
        source_pins: PortPins,
        sink_pins: PortPins,
        source_component: Option<ComponentRefId>,
        sink_component: Option<ComponentRefId>,
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

pub struct SourceSet(PortPins, Option<ComponentRefId>);
pub struct SourceUnset;
pub struct SinkSet(PortPins, Option<ComponentRefId>);
pub struct SinkUnset;

pub struct ConnectionBuilder<'m, Src, Snk> {
    module: &'m mut Module,
    component: ComponentId,
    source: Src,
    sink: Snk,
    kind: Option<ConnectionKind>,
}

impl<'m> ConnectionBuilder<'m, SourceUnset, SinkUnset> {
    pub fn new(module: &'m mut Module, component: ComponentKey) -> Self {
        Self {
            module,
            component: component.0,
            source: SourceUnset,
            sink: SinkUnset,
            kind: None,
        }
    }
}

impl<'m, Snk> ConnectionBuilder<'m, SourceUnset, Snk> {
    pub fn set_source(
        self,
        pins: PortPins,
        component: Option<ComponentRefKey>,
    ) -> ConnectionBuilder<'m, SourceSet, Snk> {
        ConnectionBuilder {
            module: self.module,
            component: self.component,
            source: SourceSet(pins, component.map(|c| c.0)),
            sink: self.sink,
            kind: self.kind,
        }
    }
}

impl<'m, Src> ConnectionBuilder<'m, Src, SinkUnset> {
    pub fn set_sink(
        self,
        pins: PortPins,
        component: Option<ComponentRefKey>,
    ) -> ConnectionBuilder<'m, Src, SinkSet> {
        ConnectionBuilder {
            module: self.module,
            component: self.component,
            source: self.source,
            sink: SinkSet(pins, component.map(|c| c.0)),
            kind: self.kind,
        }
    }
}

impl<'m, Src, Snk> ConnectionBuilder<'m, Src, Snk> {
    pub fn set_kind(&mut self, kind: ConnectionKind) {
        self.kind = Some(kind);
    }

    pub fn kind_is_set(&self) -> bool {
        self.kind.is_some()
    }
}

impl<'m> ConnectionBuilder<'m, SourceSet, SinkSet> {
    pub fn finish(self) {
        let kind = self.kind.unwrap_or(ConnectionKind::Direct);

        let connection =
            Connection::new(kind, self.source.0, self.sink.0, self.source.1, self.sink.1);

        self.module[self.component].connections.push(connection);
    }
}

#[derive(Debug, Error)]
pub enum ConnectionBuildError {
    #[error("connection must have a {0}")]
    MissingField(&'static str),
    #[error(r#"undefined port "{port}" connected"#)]
    UndefinedPort { port: String },
    #[error(r#"undefined component "{reference}" connected"#)]
    UndefinedReference { reference: String },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct Signature {
    pub pins: WeakPortPins,
    pub component: Option<String>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct WeakConnection {
    pub kind: ConnectionKind,
    pub source: Signature,
    pub sink: Signature,
}

pub struct WeakSourceSet(WeakPortPins, Option<String>);
pub struct WeakSourceUnset;
pub struct WeakSinkSet(WeakPortPins, Option<String>);
pub struct WeakSinkUnset;

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
        component: Option<String>,
    ) -> WeakConnectionBuilder<WeakSourceSet, Snk> {
        WeakConnectionBuilder {
            source: WeakSourceSet(pins, component),
            sink: self.sink,
            kind: self.kind,
        }
    }
}

impl<Src> WeakConnectionBuilder<Src, WeakSinkUnset> {
    pub fn set_sink(
        self,
        pins: WeakPortPins,
        component: Option<String>,
    ) -> WeakConnectionBuilder<Src, WeakSinkSet> {
        WeakConnectionBuilder {
            source: self.source,
            sink: WeakSinkSet(pins, component),
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
                component: self.source.1,
            },
            sink: Signature {
                pins: self.sink.0,
                component: self.sink.1,
            },
            kind: self.kind.unwrap_or(ConnectionKind::Direct),
        }
    }
}
