use std::collections::{HashMap, HashSet};

use thiserror::Error;
use ustr::{ustr, Ustr};

use super::{
    connection::{ComponentRefs, ConnectionName},
    linker::ResolvedComponent,
    port::PortPins,
    prelude::*,
};

impl std::error::Error for ConnectionName {}

#[derive(Debug, Error)]
pub enum Error {
    #[error(r#"component "{component}" already in "{module}""#)]
    ComponentExists { module: String, component: String },
    #[error(r#"port "{port}" already in "{component}""#)]
    PortExists { component: String, port: String },
    #[error(r#"component "{reference}" already referenced in "{component}""#)]
    ReferenceExists {
        component: String,
        reference: String,
    },
    #[error(r#""{source}" already driving "{sink}""#)]
    SourceCollision {
        source: Box<ConnectionName>,
        sink: Box<ConnectionName>,
    },
    #[error(r#""{sink}" already driven by "{source}""#)]
    SinkCollision {
        source: Box<ConnectionName>,
        sink: Box<ConnectionName>,
    },
}

impl Error {
    pub fn component_exists(module: &str, component: &str) -> Self {
        Self::ComponentExists {
            module: module.to_string(),
            component: component.to_string(),
        }
    }

    pub fn port_exists(component: &str, port: &str) -> Self {
        Self::PortExists {
            component: component.to_string(),
            port: port.to_string(),
        }
    }

    pub fn reference_exists(component: &str, reference: &str) -> Self {
        Self::ReferenceExists {
            component: component.to_string(),
            reference: reference.to_string(),
        }
    }

    pub fn source_collision(source: ConnectionName, sink: ConnectionName) -> Self {
        Self::SourceCollision {
            source: Box::new(source),
            sink: Box::new(sink),
        }
    }

    pub fn sink_collision(source: ConnectionName, sink: ConnectionName) -> Self {
        Self::SinkCollision {
            source: Box::new(source),
            sink: Box::new(sink),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub(super) struct CheckComponent {
    ports: HashSet<Ustr>,
    references: HashSet<Ustr>,
}

impl CheckComponent {
    pub fn new(module: &Module, component: ComponentId) -> Self {
        let component = component.bind(module);
        let ports = HashSet::from_iter(component.ports().map(|port| ustr(port.name())));

        let references = HashSet::from_iter(
            component
                .references()
                .map(|reference| ustr(reference.alias_or_name())),
        );

        Self { ports, references }
    }
}

impl From<ResolvedComponent> for CheckComponent {
    fn from(component: ResolvedComponent) -> Self {
        Self {
            ports: component.ports,
            references: component.references,
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct PinRecord {
    port: PortId,
    pin: u32,
    reference: Option<(ComponentRefId, u32)>,
}

#[derive(Clone, Debug, Default)]
struct PinUsage {
    source: Option<ConnectionId>,
    sink: Option<ConnectionId>,
}

#[derive(Default)]
struct Connectivity {
    pin_indices: HashMap<PinRecord, u64>,
    pin_usage: Vec<PinUsage>,
    pin_index_end: u64,
}

impl Connectivity {
    fn get_record_index(&mut self, record: PinRecord) -> u64 {
        *self.pin_indices.entry(record).or_insert_with(|| {
            let index = self.pin_index_end;
            self.pin_index_end += 1;
            index
        })
    }

    fn ensure_usage(&mut self) {
        self.pin_usage
            .resize_with(self.pin_index_end as usize, PinUsage::default);
    }

    fn get_usage_mut(&mut self, pin: u64) -> &mut PinUsage {
        assert!(pin < self.pin_index_end);
        self.ensure_usage();
        &mut self.pin_usage[pin as usize]
    }

    fn collect_pins(
        &mut self,
        module: &Module,
        port: &PortPins,
        reference: Option<&ComponentRefs>,
    ) -> Vec<PinRecord> {
        if let Some(reference) = reference {
            reference
                .range(module)
                .flat_map(|reference_i| {
                    port.range(module).map(move |pin| PinRecord {
                        port: port.id(),
                        pin,
                        reference: Some((reference.id(), reference_i)),
                    })
                })
                .collect()
        } else {
            port.range(module)
                .map(|pin| PinRecord {
                    port: (&port).id(),
                    pin,
                    reference: None,
                })
                .collect()
        }
    }

    fn collect_records(
        &mut self,
        module: &Module,
        connection: ConnectionId,
    ) -> (Vec<PinRecord>, Vec<PinRecord>) {
        let connection = module.lookup(connection);

        (
            self.collect_pins(
                module,
                &connection.source_pins,
                connection.source_component.as_ref(),
            ),
            self.collect_pins(
                module,
                &connection.sink_pins,
                connection.sink_component.as_ref(),
            ),
        )
    }

    fn register_connection(&mut self, module: &Module, connection: ConnectionId) -> Result<()> {
        let (source_pins, sink_pins) = self.collect_records(module, connection);

        for pin in source_pins {
            let index = self.get_record_index(pin);
            let usage = self.get_usage_mut(index);

            if let Some(existing) = usage.source {
                let existing = existing.bind(module);
                let source = existing.source_name_or_default();
                let sink = existing.sink_name_or_default();
                return Err(Error::source_collision(source, sink));
            } else {
                usage.source = Some(connection);
            }
        }

        for pin in sink_pins {
            let index = self.get_record_index(pin);
            let usage = self.get_usage_mut(index);

            if let Some(existing) = usage.sink {
                let existing = existing.bind(module);
                let source = existing.source_name_or_default();
                let sink = existing.sink_name_or_default();
                return Err(Error::sink_collision(source, sink));
            } else {
                usage.sink = Some(connection);
            }
        }

        Ok(())
    }

    fn new(module: &Module) -> Result<Self> {
        let mut connections = Self::default();

        module
            .components()
            .flat_map(|component| component.connections())
            .try_for_each(|connection| connections.register_connection(module, connection.id()))?;

        Ok(connections)
    }
}

#[derive(Default)]
pub struct Checker {
    pub(super) components: HashMap<Ustr, CheckComponent>,
    connections: Connectivity,
}

impl TryFrom<&Module> for Checker {
    type Error = Error;

    fn try_from(module: &Module) -> Result<Self> {
        let components = HashMap::from_iter(module.components().map(|component| {
            (
                ustr(component.name()),
                CheckComponent::new(module, component.unbind()),
            )
        }));

        let connections = Connectivity::new(module)?;

        Ok(Self {
            components,
            connections,
        })
    }
}

impl Checker {
    fn get_component_checker(&self, module: &Module, component: ComponentId) -> &CheckComponent {
        self.components
            .get(&module.lookup(component).name)
            .expect("component should be in checker")
    }

    fn get_component_checker_mut(
        &mut self,
        module: &Module,
        component: ComponentId,
    ) -> &mut CheckComponent {
        self.components
            .get_mut(&module.lookup(component).name)
            .expect("component should be in checker")
    }

    pub fn register_component<C: ComponentAccess>(
        &mut self,
        module: &Module,
        component: C,
    ) -> Result<()> {
        let component = component.id();
        let name = module.lookup(component).name;
        let checker = CheckComponent::new(module, component);

        if self.components.insert(name, checker).is_none() {
            Ok(())
        } else {
            Err(Error::component_exists(module.name(), &name))
        }
    }

    pub fn register_port<C: ComponentAccess>(
        &mut self,
        module: &Module,
        component: C,
        port: PortId,
    ) -> Result<()> {
        let component = component.id();
        let name = port.bind(module).data().name;
        let checker = self.get_component_checker_mut(module, component);

        if checker.ports.insert(name) {
            Ok(())
        } else {
            let component = module.lookup(component).name;
            Err(Error::port_exists(&component, &name))
        }
    }

    pub fn register_reference<C: ComponentAccess, R: ComponentRefAccess>(
        &mut self,
        module: &Module,
        component: C,
        reference: R,
    ) -> Result<()> {
        let component = component.id();
        let reference = ustr(reference.bind(module).alias_or_name());
        let checker = self.get_component_checker_mut(module, component);

        if checker.references.insert(reference) {
            Ok(())
        } else {
            let component = module.lookup(component).name;
            Err(Error::reference_exists(&component, &reference))
        }
    }

    pub fn register_connection<C: ConnectionAccess>(
        &mut self,
        module: &Module,
        connection: C,
    ) -> Result<()> {
        self.connections
            .register_connection(module, connection.id())
    }

    pub fn ensure_no_existing_component(&self, module: &Module, component: &str) -> Result<()> {
        if self.components.contains_key(&ustr(component)) {
            Err(Error::component_exists(module.name(), component))
        } else {
            Ok(())
        }
    }

    pub fn ensure_no_existing_port<C: ComponentAccess>(
        &self,
        module: &Module,
        component: C,
        port: &str,
    ) -> Result<()> {
        let component = component.bind(module);
        let checker = self.get_component_checker(module, component.id());

        if checker.ports.contains(&ustr(port)) {
            Err(Error::port_exists(component.name(), port))
        } else {
            Ok(())
        }
    }

    pub fn ensure_no_existing_reference<C: ComponentAccess>(
        &self,
        module: &Module,
        component: C,
        reference: &str,
    ) -> Result<()> {
        let component = component.bind(module);
        let checker = self.get_component_checker(module, component.id());

        if checker.references.contains(&ustr(reference)) {
            Err(Error::reference_exists(component.name(), reference))
        } else {
            Ok(())
        }
    }
}
