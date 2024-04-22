use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};
use slotmap::SlotMap;

use super::{
    component::{self, ComponentData},
    connection::{ComponentRefs, WeakConnectionBuilder},
    module,
    port::{pin_range, PinRange},
    prelude::*,
    reference::{reference_range, ComponentWeakRef, ReferenceRange},
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
            state.serialize_entry(
                data.name.as_str(),
                &SerializeComponent::new(self.module, component),
            )?;
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

impl Serialize for SerializePorts<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.ports.len()))?;

        for &port in self.ports {
            let port = port.bind(self.module);
            state.serialize_entry(port.name(), port.data())?;
        }

        state.end()
    }
}

struct SerializeReferences<'a, 'm> {
    module: &'m Module,
    references: &'a Vec<ComponentRefId>,
}

impl SerializeReferences<'_, '_> {
    pub fn iter_unnamed(&self) -> impl Clone + Iterator<Item = &ComponentRefId> {
        self.references
            .iter()
            .filter(|&reference| self.module.lookup(*reference).alias.is_none())
    }

    pub fn should_serialize(&self) -> bool {
        self.iter_unnamed().next().is_some()
    }
}

impl Serialize for SerializeReferences<'_, '_> {
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
            let reference = reference.bind(self.module);

            state.serialize_element(&ComponentWeakRef {
                component: reference.component().data().name,
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
            .filter(|&reference| self.module.lookup(*reference).alias.is_some())
    }

    pub fn should_serialize(&self) -> bool {
        self.iter_named().next().is_some()
    }
}

impl Serialize for SerializeNamedReferences<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let named_references = self.iter_named();

        let len = named_references.clone().count();
        let mut state = serializer.serialize_map(Some(len))?;

        for &reference in named_references {
            let reference = reference.bind(self.module);
            let alias = reference.alias().expect("reference should have an alias");

            state.serialize_entry(
                alias,
                &ComponentWeakRef {
                    component: reference.component().data().name,
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
    connections: &'a Vec<ConnectionId>,
}

impl SerializeConnections<'_, '_> {
    pub fn should_serialize(&self) -> bool {
        !self.connections.is_empty()
    }
}

impl Serialize for SerializeConnections<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.connections.len()))?;
        let connections = self
            .connections
            .iter()
            .map(|connection| connection.bind(self.module));

        for connection in connections {
            let source_pins = connection.source_pins().to_weak(self.module);
            let sink_pins = connection.sink_pins().to_weak(self.module);

            let select_component = |component: &ComponentRefs| {
                (
                    Some(component.reference(self.module).alias_or_name()),
                    component.get_start(),
                    component.get_end(),
                )
            };

            let (source_component, source_start, source_end) = connection
                .source_component()
                .map(select_component)
                .unwrap_or((None, None, None));

            let (sink_component, sink_start, sink_end) = connection
                .sink_component()
                .map(select_component)
                .unwrap_or((None, None, None));

            let mut builder = WeakConnectionBuilder::new()
                .set_source(source_pins, source_component, source_start, source_end)
                .set_sink(sink_pins, sink_component, sink_start, sink_end);

            builder.set_kind(connection.kind());
            state.serialize_element(&builder.finish())?;
        }

        state.end()
    }
}

struct SerializeComponent<'m> {
    component: Component<'m>,
}

impl<'m> SerializeComponent<'m> {
    pub(crate) fn new(module: &'m Module, component: ComponentId) -> Self {
        Self {
            component: component.bind(module),
        }
    }
}

impl Serialize for SerializeComponent<'_> {
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
        let start = self.get_start();
        let end = self.get_end();

        let len = match (start, end) {
            (Some(_), Some(_)) => 2,
            (Some(_), None) => 1,
            (None, Some(_)) => 1,
            (None, None) => 0,
        };

        let mut state = serializer.serialize_map(Some(len))?;

        if let Some(start) = start {
            state.serialize_entry(pin_range::FIELDS[pin_range::PORT_START], &start)?;
        }

        if let Some(end) = end {
            state.serialize_entry(pin_range::FIELDS[pin_range::PORT_END], &end)?;
        }

        state.end()
    }
}

impl Serialize for ReferenceRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let start = self.get_start();
        let end = self.get_end();

        let len = match (start, end) {
            (Some(_), Some(_)) => 2,
            (Some(_), None) => 1,
            (None, Some(_)) => 1,
            (None, None) => 0,
        };

        let mut state = serializer.serialize_map(Some(len))?;

        if let Some(start) = start {
            state.serialize_entry(
                reference_range::FIELDS[reference_range::REFERENCE_START],
                &start,
            )?;
        }

        if let Some(end) = end {
            state.serialize_entry(
                reference_range::FIELDS[reference_range::REFERENCE_END],
                &end,
            )?;
        }

        state.end()
    }
}
