use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};
use slotmap::SlotMap;

use super::{
    component::{self, ComponentData},
    connection::WeakConnectionBuilder,
    module,
    port::{PinRange, WeakPortPins},
    prelude::*,
    reference::ComponentWeakRef,
};

struct SerializeComponents<'a, 'm> {
    module: &'m Module,
    components: &'a SlotMap<ComponentId, ComponentData>,
}

impl<'a, 'm> Serialize for SerializeComponents<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.components.len()))?;

        for (component, data) in self.components.iter() {
            state.serialize_entry(&data.name, &SerializeComponent::new(self.module, component))?;
        }

        state.end()
    }
}

impl Serialize for Module {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Module", module::FIELDS.len())?;
        state.serialize_field(module::FIELDS[module::NAME], self.name())?;

        state.serialize_field(
            module::FIELDS[module::COMPONENTS],
            &SerializeComponents {
                module: self,
                components: &self.components,
            },
        )?;

        state.end()
    }
}
struct SerializePorts<'a, 'm> {
    module: &'m Module,
    ports: &'a Vec<PortId>,
}

impl SerializePorts<'_, '_> {
    pub fn should_serialize(&self) -> bool {
        !self.ports.is_empty()
    }
}

impl<'m> Serialize for SerializePorts<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.ports.len()))?;

        for &port in self.ports {
            let port = Port::new(self.module, port);
            state.serialize_entry(port.name(), port.data())?;
        }

        state.end()
    }
}

struct SerializeReferences<'a, 'm> {
    module: &'m Module,
    references: &'a Vec<ComponentRefId>,
}

impl<'m> SerializeReferences<'_, 'm> {
    pub fn iter_unnamed(&self) -> impl Clone + Iterator<Item = &ComponentRefId> {
        self.references
            .iter()
            .filter(|&reference| self.module[*reference].alias.is_none())
    }

    pub fn should_serialize(&self) -> bool {
        self.iter_unnamed().next().is_some()
    }
}

impl<'m> Serialize for SerializeReferences<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let unnamed_references = self.iter_unnamed();

        let len = unnamed_references.clone().count();
        let mut state = serializer.serialize_seq(Some(len))?;

        if len == 0 {
            return state.end();
        }

        for &reference in unnamed_references {
            let reference = ComponentRef::new(self.module, reference);

            state.serialize_element(&ComponentWeakRef {
                component: reference.component().name().to_string(),
                alias: None,
                n_instances: reference.n_instances(),
            })?;
        }

        state.end()
    }
}

struct SerializeNamedReferences<'a, 'm> {
    module: &'m Module,
    references: &'a Vec<ComponentRefId>,
}

impl SerializeNamedReferences<'_, '_> {
    pub fn iter_named(&self) -> impl Clone + Iterator<Item = &ComponentRefId> {
        self.references
            .iter()
            .filter(|&reference| self.module[*reference].alias.is_some())
    }

    pub fn should_serialize(&self) -> bool {
        self.iter_named().next().is_some()
    }
}

impl<'m> Serialize for SerializeNamedReferences<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let named_references = self.iter_named();

        let len = named_references.clone().count();
        let mut state = serializer.serialize_map(Some(len))?;

        for &reference in named_references {
            let reference = ComponentRef::new(self.module, reference);
            let alias = reference.alias().expect("reference should have an alias");

            state.serialize_entry(
                alias,
                &ComponentWeakRef {
                    component: reference.component().name().to_string(),
                    alias: None,
                    n_instances: reference.n_instances(),
                },
            )?;
        }

        state.end()
    }
}

struct SerializeConnections<'a, 'm> {
    module: &'m Module,
    connections: &'a Vec<Connection>,
}

impl SerializeConnections<'_, '_> {
    pub fn should_serialize(&self) -> bool {
        !self.connections.is_empty()
    }
}

impl<'m> Serialize for SerializeConnections<'_, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.connections.len()))?;

        for connection in self.connections {
            let source_pins = WeakPortPins {
                port: connection.source_pins.port(self.module).name().to_string(),
                range: connection.source_pins.range.clone(),
            };

            let sink_pins = WeakPortPins {
                port: connection.sink_pins.port(self.module).name().to_string(),
                range: connection.sink_pins.range.clone(),
            };

            let source_component = connection.source_component.map(|component| {
                ComponentRef::new(self.module, component)
                    .alias_or_name()
                    .to_string()
            });

            let sink_component = connection.sink_component.map(|component| {
                ComponentRef::new(self.module, component)
                    .alias_or_name()
                    .to_string()
            });

            let mut builder = WeakConnectionBuilder::new()
                .set_source(source_pins, source_component)
                .set_sink(sink_pins, sink_component);

            builder.set_kind(connection.kind);
            state.serialize_element(&builder.finish())?;
        }

        state.end()
    }
}

pub(crate) struct SerializeComponent<'m> {
    component: Component<'m>,
}

impl<'m> SerializeComponent<'m> {
    pub(crate) fn new(module: &'m Module, component: ComponentId) -> Self {
        Self {
            component: Component::new(module, component),
        }
    }
}

impl<'m> Serialize for SerializeComponent<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Component", component::FIELDS.len())?;

        let ports = SerializePorts {
            module: self.component.module(),
            ports: &self.component.data().ports,
        };

        if ports.should_serialize() {
            state.serialize_field(component::FIELDS[component::PORTS], &ports)?;
        }

        let references = SerializeReferences {
            module: self.component.module(),
            references: &self.component.data().references,
        };

        if references.should_serialize() {
            state.serialize_field(component::FIELDS[component::REFERENCES], &references)?;
        }

        let named_references = SerializeNamedReferences {
            module: self.component.module(),
            references: &self.component.data().references,
        };

        if named_references.should_serialize() {
            state.serialize_field(
                component::FIELDS[component::NAMED_REFERENCES],
                &named_references,
            )?;
        }

        let connections = SerializeConnections {
            module: self.component.module(),
            connections: &self.component.data().connections,
        };

        if connections.should_serialize() {
            state.serialize_field(component::FIELDS[component::CONNECTIONS], &connections)?;
        }

        if self.component.class().is_some() {
            state.serialize_field(component::FIELDS[component::CLASS], &self.component.class())?;
        }

        state.end()
    }
}

impl Serialize for PinRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let port_start = self.get_start();
        let port_end = self.get_end();
        let mut len = 0;

        if port_start.is_some() {
            len += 1;
        }

        if port_end.is_some() {
            len += 1;
        }

        let mut state = serializer.serialize_map(Some(len))?;

        if let Some(port_start) = port_start {
            state.serialize_entry("port_start", &port_start)?;
        }

        if let Some(port_end) = port_end {
            state.serialize_entry("port_end", &port_end)?;
        }

        state.end()
    }
}
