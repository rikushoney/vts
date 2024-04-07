use std::ops::Range;

use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConnectionKind {
    Complete,
    Direct,
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

    // pub fn source_component<'m>(&self, module: &'m Module) -> Option<ComponentRef<'m>> {
    //     self.source_component
    //         .map(|source_component| source_component.to_reference(module))
    // }

    // pub fn sink_component<'m>(&self, module: &'m Module) -> Option<ComponentRef<'m>> {
    //     self.sink_component
    //         .map(|sink_component| sink_component.to_reference(module))
    // }

    // pub fn source_port<'m>(
    //     &self,
    //     module: &'m Module,
    //     component: &Component<'m>,
    // ) -> Option<Port<'m>> {
    //     if let Some(source_component) = self.source_component {
    //         let source_component = source_component.to_component(module);
    //         source_component
    //             .ports()
    //             .find(|port| port.name() == self.source_pins.port(module).name())
    //     } else {
    //         component
    //             .ports()
    //             .find(|port| port.name() == self.source_pins.port(module).name())
    //     }
    // }

    // pub fn sink_port<'m>(&self, module: &'m Module, component: &Component<'m>) -> Option<Port<'m>> {
    //     if let Some(sink_component) = self.sink_component {
    //         let sink_component = sink_component.to_component(module);
    //         sink_component
    //             .ports()
    //             .find(|port| port.name() == self.sink_pins.port(module).name())
    //     } else {
    //         component
    //             .ports()
    //             .find(|port| port.name() == self.sink_pins.port(module).name())
    //     }
    // }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WeakConnection {
    pub kind: ConnectionKind,
    pub(crate) source_pins: WeakPortPins,
    pub(crate) source_component: Option<String>,
    pub(crate) sink_pins: WeakPortPins,
    pub(crate) sink_component: Option<String>,
}

pub struct ConnectionBuilder<'m> {
    component: &'m mut ComponentData,
    kind: Option<ConnectionKind>,
    source: Option<(PortPins, Option<ComponentRefId>)>,
    sink: Option<(PortPins, Option<ComponentRefId>)>,
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

impl<'m> ConnectionBuilder<'m> {
    pub(super) fn new(component: &'m mut ComponentData) -> Self {
        Self {
            component,
            kind: None,
            source: None,
            sink: None,
        }
    }

    pub fn set_kind(&mut self, kind: ConnectionKind) -> &mut Self {
        self.kind = Some(kind);
        self
    }

    pub(crate) fn set_source(
        &mut self,
        pins: PortPins,
        component: Option<ComponentRefId>,
    ) -> &mut Self {
        self.source = Some((pins, component));
        self
    }

    pub(crate) fn set_sink(
        &mut self,
        pins: PortPins,
        component: Option<ComponentRefId>,
    ) -> &mut Self {
        self.sink = Some((pins, component));
        self
    }

    pub fn is_kind_set(&self) -> bool {
        self.kind.is_some()
    }

    pub fn is_source_set(&self) -> bool {
        self.source.is_some()
    }

    pub fn is_sink_set(&self) -> bool {
        self.sink.is_some()
    }

    // pub fn finish(self) -> Result<&'m Connection, ConnectionBuildError> {
    //     let kind = self
    //         .kind
    //         .ok_or(ConnectionBuildError::MissingField("kind"))?;
    //     let source = self
    //         .source
    //         .ok_or(ConnectionBuildError::MissingField("source"))?;
    //     let sink = self
    //         .sink
    //         .ok_or(ConnectionBuildError::MissingField("sink"))?;

    //     let connections = &mut self.component.connections;
    //     let i = connections.len();
    //     connections.push(Connection::new(kind, source.0, sink.0, source.1, sink.1));

    //     Ok(&connections[i])
    // }
}

pub struct WeakConnectionBuilder<'a, 'm> {
    builder: &'a mut ComponentBuilder<'m>,
    kind: Option<ConnectionKind>,
    source: Option<(WeakPortPins, Option<String>)>,
    sink: Option<(WeakPortPins, Option<String>)>,
}

impl<'a, 'm> WeakConnectionBuilder<'a, 'm> {
    pub(super) fn new(builder: &'a mut ComponentBuilder<'m>) -> Self {
        Self {
            builder,
            kind: None,
            source: None,
            sink: None,
        }
    }

    pub fn set_kind(&mut self, kind: ConnectionKind) -> &mut Self {
        self.kind = Some(kind);
        self
    }

    // pub fn set_source(
    //     &mut self,
    //     port: &str,
    //     range: Range<u32>,
    //     component: Option<&str>,
    // ) -> &mut Self {
    //     let module = &mut self.builder.module;
    //     let port = module.strings.entry(port);
    //     let pins = WeakPortPins::new(port, range);
    //     let component = component.map(|component| module.strings.entry(component));
    //     self.source = Some((pins, component));
    //     self
    // }

    // pub fn set_sink(
    //     &mut self,
    //     port: &str,
    //     range: Range<u32>,
    //     component: Option<&str>,
    // ) -> &mut Self {
    //     let module = &mut self.builder.module;
    //     let port = module.strings.entry(port);
    //     let pins = WeakPortPins::new(port, range);
    //     let component = component.map(|component| self.builder.module.strings.entry(component));
    //     self.sink = Some((pins, component));
    //     self
    // }

    pub fn is_kind_set(&self) -> bool {
        self.kind.is_some()
    }

    pub fn is_source_set(&self) -> bool {
        self.source.is_some()
    }

    pub fn is_sink_set(&self) -> bool {
        self.sink.is_some()
    }

    // pub fn finish(self) -> Result<&'a WeakConnection, ConnectionBuildError> {
    //     let kind = self
    //         .kind
    //         .ok_or(ConnectionBuildError::MissingField("kind"))?;
    //     let source = self
    //         .source
    //         .ok_or(ConnectionBuildError::MissingField("source"))?;
    //     let sink = self
    //         .sink
    //         .ok_or(ConnectionBuildError::MissingField("sink"))?;

    //     let connections = &mut self.builder.unresolved_connections;
    //     let i = connections.len();
    //     connections.push(WeakConnection {
    //         kind,
    //         source_pins: source.0,
    //         source_component: source.1,
    //         sink_pins: sink.0,
    //         sink_component: sink.1,
    //     });

    //     Ok(&connections[i])
    // }
}
