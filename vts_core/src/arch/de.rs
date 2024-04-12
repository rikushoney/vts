use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

use super::{
    component::{self, ComponentBuilder, ComponentKey},
    connection::WeakConnection,
    module,
    port::{self, pin_range, PinRange, PortBuilder},
    prelude::*,
    reference::{self, ComponentWeakRef},
};

struct DeserializeComponents<'a, 'm> {
    module: &'m mut Module,
    linker: &'a mut Linker,
}

impl<'a, 'de, 'm> Visitor<'de> for DeserializeComponents<'a, 'm> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a dict of {}", module::FIELDS[module::COMPONENTS])
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(component) = map.next_key()? {
            map.next_value_seed(ComponentSeed::new(self.module, component, self.linker))?;
        }

        Ok(())
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for DeserializeComponents<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Deserialize<'de> for Module {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModuleVisitor;

        impl<'de> Visitor<'de> for ModuleVisitor {
            type Value = Module;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a module description")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case")]
                enum Field {
                    Name,
                    Components,
                }

                const PLACEHOLDER_NAME: &str = "__vts_placeholder_module_name";
                let mut module = Module::new(PLACEHOLDER_NAME);
                let mut linker = Linker::new();

                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Name => {
                            if module.name != PLACEHOLDER_NAME {
                                return Err(de::Error::duplicate_field(
                                    module::FIELDS[module::NAME],
                                ));
                            }

                            module.rename(map.next_value()?);
                        }
                        Field::Components => {
                            if !module.components.is_empty() {
                                return Err(de::Error::duplicate_field(
                                    module::FIELDS[module::COMPONENTS],
                                ));
                            }

                            map.next_value_seed(DeserializeComponents {
                                module: &mut module,
                                linker: &mut linker,
                            })?;
                        }
                    }
                }

                if module.name == PLACEHOLDER_NAME {
                    return Err(de::Error::missing_field(module::FIELDS[module::NAME]));
                }

                linker.resolve(&mut module).map_err(de::Error::custom)?;

                Ok(module)
            }
        }

        deserializer.deserialize_struct("Module", module::FIELDS, ModuleVisitor)
    }
}

struct DeserializePorts<'m> {
    module: &'m mut Module,
    parent: ComponentId,
}

impl<'de, 'm> Visitor<'de> for DeserializePorts<'m> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a dict of {}", component::FIELDS[component::PORTS])
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(port) = map.next_key::<String>()? {
            map.next_value_seed(PortSeed::new(self.module, self.parent, port))?;
        }

        Ok(())
    }
}

impl<'de, 'm> DeserializeSeed<'de> for DeserializePorts<'m> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

struct DeserializeReferences<'a> {
    linker: &'a mut Linker,
    parent: ComponentId,
}

impl<'a, 'de> Visitor<'de> for DeserializeReferences<'a> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a dict of {}", component::FIELDS[component::REFERENCES])
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(reference) = seq.next_element_seed(DeserializeComponentWeakRef::Unnamed)? {
            self.linker
                .register_reference(ComponentKey::new(self.parent), reference);
        }

        Ok(())
    }
}

impl<'a, 'de> DeserializeSeed<'de> for DeserializeReferences<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct DeserializeNamedReferences<'a> {
    linker: &'a mut Linker,
    parent: ComponentId,
}

impl<'a, 'de> Visitor<'de> for DeserializeNamedReferences<'a> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "a dict of {}",
            component::FIELDS[component::NAMED_REFERENCES]
        )
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(alias) = map.next_key()? {
            let reference = map.next_value_seed(DeserializeComponentWeakRef::Named(alias))?;
            self.linker
                .register_reference(ComponentKey::new(self.parent), reference);
        }

        Ok(())
    }
}

impl<'a, 'de> DeserializeSeed<'de> for DeserializeNamedReferences<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

struct DeserializeConnections<'a> {
    parent: ComponentId,
    linker: &'a mut Linker,
}

impl<'a, 'de> Visitor<'de> for DeserializeConnections<'a> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a list of {}", component::FIELDS[component::CONNECTIONS])
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(connection) = seq.next_element::<WeakConnection>()? {
            self.linker
                .register_connection(ComponentKey::new(self.parent), connection);
        }

        Ok(())
    }
}

impl<'a, 'de> DeserializeSeed<'de> for DeserializeConnections<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct ComponentSeed<'a, 'm> {
    module: &'m mut Module,
    name: String,
    linker: &'a mut Linker,
}

impl<'a, 'm> ComponentSeed<'a, 'm> {
    pub(crate) fn new(module: &'m mut Module, name: String, linker: &'a mut Linker) -> Self {
        Self {
            module,
            name,
            linker,
        }
    }
}

impl<'a, 'de, 'm> Visitor<'de> for ComponentSeed<'a, 'm> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a component description")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Field {
            Ports,
            References,
            NamedReferences,
            Connections,
            Class,
        }

        let component = ComponentBuilder::new(self.module)
            .set_name(&self.name)
            .finish()
            .1;

        let mut ports = false;
        let mut references = false;
        let mut named_references = false;
        let mut connections = false;
        let mut class: Option<ComponentClass> = None;

        while let Some(field) = map.next_key()? {
            match field {
                Field::Ports => {
                    if ports {
                        return Err(de::Error::duplicate_field(
                            component::FIELDS[component::PORTS],
                        ));
                    }

                    map.next_value_seed(DeserializePorts {
                        module: self.module,
                        parent: component,
                    })?;

                    ports = true;
                }
                Field::References => {
                    if references {
                        return Err(de::Error::duplicate_field(
                            component::FIELDS[component::REFERENCES],
                        ));
                    }

                    map.next_value_seed(DeserializeReferences {
                        parent: component,
                        linker: self.linker,
                    })?;

                    references = true;
                }
                Field::NamedReferences => {
                    if named_references {
                        return Err(de::Error::duplicate_field(
                            component::FIELDS[component::NAMED_REFERENCES],
                        ));
                    }

                    map.next_value_seed(DeserializeNamedReferences {
                        parent: component,
                        linker: self.linker,
                    })?;

                    named_references = true;
                }
                Field::Connections => {
                    if connections {
                        return Err(de::Error::duplicate_field(
                            component::FIELDS[component::CONNECTIONS],
                        ));
                    }

                    map.next_value_seed(DeserializeConnections {
                        parent: component,
                        linker: self.linker,
                    })?;

                    connections = true;
                }
                Field::Class => {
                    if class.is_some() {
                        return Err(de::Error::duplicate_field(
                            component::FIELDS[component::CLASS],
                        ));
                    }

                    class = Some(map.next_value()?);
                }
            }
        }

        self.module[component].class = class;
        Ok(())
    }
}

impl<'a, 'de, 'm> DeserializeSeed<'de> for ComponentSeed<'a, 'm> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Component", component::FIELDS, self)
    }
}

struct PortSeed<'m> {
    module: &'m mut Module,
    parent: ComponentId,
    name: String,
}

impl<'m> PortSeed<'m> {
    pub(crate) fn new(module: &'m mut Module, parent: ComponentId, name: String) -> Self {
        Self {
            module,
            parent,
            name,
        }
    }
}

impl<'de, 'm> Visitor<'de> for PortSeed<'m> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a port description")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Field {
            Kind,
            NPins,
            Class,
        }

        let mut kind: Option<PortKind> = None;
        let mut n_pins: Option<u32> = None;
        let mut class: Option<PortClass> = None;

        while let Some(field) = map.next_key()? {
            match field {
                Field::Kind => {
                    if kind.is_some() {
                        return Err(de::Error::duplicate_field(port::FIELDS[port::KIND]));
                    }

                    kind = Some(map.next_value()?);
                }
                Field::NPins => {
                    if n_pins.is_some() {
                        return Err(de::Error::duplicate_field(port::FIELDS[port::N_PINS]));
                    }

                    n_pins = Some(map.next_value()?);
                }
                Field::Class => {
                    if class.is_some() {
                        return Err(de::Error::duplicate_field(port::FIELDS[port::CLASS]));
                    }

                    class = Some(map.next_value()?);
                }
            }
        }

        let kind = kind.ok_or(de::Error::missing_field(port::FIELDS[port::KIND]))?;

        let mut builder = PortBuilder::new(self.module, ComponentKey(self.parent))
            .set_name(&self.name)
            .set_kind(kind);

        if let Some(n_pins) = n_pins {
            builder.set_n_pins(n_pins);
        }

        if let Some(class) = class {
            builder.set_class(class);
        }

        builder.finish();
        Ok(())
    }
}

impl<'de, 'm> DeserializeSeed<'de> for PortSeed<'m> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Port", port::FIELDS, self)
    }
}

impl<'de> Deserialize<'de> for PinRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PinRangeVisitor;

        impl<'de> Visitor<'de> for PinRangeVisitor {
            type Value = PinRange;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a pin range")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                enum Field {
                    #[serde(rename = "port_start")]
                    PortStart,
                    #[serde(rename = "port_end")]
                    PortEnd,
                }

                let mut port_start: Option<u32> = None;
                let mut port_end: Option<u32> = None;

                while let Some(field) = map.next_key()? {
                    match field {
                        Field::PortStart => {
                            if port_start.is_some() {
                                return Err(de::Error::duplicate_field(
                                    pin_range::FIELDS[pin_range::PORT_START],
                                ));
                            }

                            port_start = Some(map.next_value()?);
                        }
                        Field::PortEnd => {
                            if port_end.is_some() {
                                return Err(de::Error::duplicate_field(
                                    pin_range::FIELDS[pin_range::PORT_END],
                                ));
                            }

                            port_end = Some(map.next_value()?);
                        }
                    }
                }

                Ok(PinRange::new(port_start, port_end))
            }
        }

        deserializer.deserialize_map(PinRangeVisitor)
    }
}

enum DeserializeComponentWeakRef {
    Named(String),
    Unnamed,
}

impl<'de> Visitor<'de> for DeserializeComponentWeakRef {
    type Value = ComponentWeakRef;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a component reference description")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Field {
            Component,
            NInstances,
        }

        let mut component: Option<String> = None;
        let mut n_instances: Option<usize> = None;

        while let Some(field) = map.next_key()? {
            match field {
                Field::Component => {
                    if component.is_some() {
                        return Err(de::Error::duplicate_field(
                            reference::FIELDS[reference::COMPONENT],
                        ));
                    }

                    component = Some(map.next_value()?);
                }
                Field::NInstances => {
                    if n_instances.is_some() {
                        return Err(de::Error::duplicate_field(
                            reference::FIELDS[reference::N_INSTANCES],
                        ));
                    }

                    n_instances = Some(map.next_value()?);
                }
            }
        }

        let component = component.ok_or(de::Error::missing_field(
            reference::FIELDS[reference::COMPONENT],
        ))?;

        let n_instances = n_instances.unwrap_or(1);

        let alias = match self {
            DeserializeComponentWeakRef::Named(alias) => Some(alias),
            DeserializeComponentWeakRef::Unnamed => None,
        };

        Ok(ComponentWeakRef {
            component,
            alias,
            n_instances,
        })
    }
}

impl<'de> DeserializeSeed<'de> for DeserializeComponentWeakRef {
    type Value = ComponentWeakRef;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("ComponentRef", reference::FIELDS, self)
    }
}
