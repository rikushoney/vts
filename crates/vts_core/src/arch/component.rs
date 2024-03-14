use std::collections::HashMap;

use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Deserialize, Serialize, Serializer,
};

use crate::arch::{
    assert_ptr_eq, ComponentId, Module, Port, PortClass, PortId, PortKind, StringId,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component<'m> {
    pub(crate) module: &'m Module<'m>,
    pub(crate) name: StringId,
    ports: HashMap<StringId, PortId>,
    references: HashMap<StringId, ComponentId>,
    pub class: Option<ComponentClass>,
}

impl<'m> Component<'m> {
    pub fn new(module: &'m mut Module, name: &str, class: Option<ComponentClass>) -> Self {
        let name = module.strings.entry(name);
        let ports = HashMap::default();
        let references = HashMap::default();

        Self {
            module,
            name,
            ports,
            references,
            class,
        }
    }

    pub fn name(&self) -> &str {
        self.module.strings.lookup(self.name)
    }

    pub fn port(&self, name: &str) -> Option<&Port<'m>> {
        let name = self.module.strings.rlookup(name)?;
        let id = *self.ports.get(&name)?;

        Some(self.module.ports.lookup(id))
    }

    pub fn add_port(
        &'m mut self,
        module: &'m mut Module<'m>,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> &Port<'m> {
        assert_ptr_eq!(module, self.module);

        let name = module.strings.entry(name);
        let port = Port::new(self, name, kind, n_pins, class);
        let id = module.ports.entry(port);
        match module.port_name_map.insert(name, id) {
            Some(_) => {
                let name = module.strings.lookup(name);
                panic!(r#""{name}" already in module"#);
            }
            None => module.ports.lookup(id),
        }
    }
}

struct PortsSerializer<'a, 'm> {
    module: &'m Module<'m>,
    ports: &'a HashMap<StringId, PortId>,
}

impl<'a, 'm> Serialize for PortsSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_map(Some(self.ports.len()))?;

        for (name, port) in self.ports {
            let name = self.module.strings.lookup(*name);
            let port = self.module.ports.lookup(*port);
            serializer.serialize_entry(name, port)?;
        }

        serializer.end()
    }
}

struct ComponentRefsSerializer<'a, 'm> {
    module: &'m Module<'m>,
    references: &'a HashMap<StringId, ComponentId>,
}

impl<'a, 'm> Serialize for ComponentRefsSerializer<'a, 'm> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(self.references.len()))?;

        #[allow(clippy::for_kv_map)]
        for (name, _component) in self.references {
            let name = self.module.strings.lookup(*name);
            serializer.serialize_element(name)?;
        }

        serializer.end()
    }
}

impl<'m> Serialize for Component<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Component", 4)?;

        let name = self.module.strings.lookup(self.name);
        serializer.serialize_field("name", name)?;

        let ports_serializer = PortsSerializer {
            module: self.module,
            ports: &self.ports,
        };
        serializer.serialize_field("ports", &ports_serializer)?;

        let component_refs_serializer = ComponentRefsSerializer {
            module: self.module,
            references: &self.references,
        };
        serializer.serialize_field("references", &component_refs_serializer)?;

        serializer.serialize_field("class", &self.class)?;

        serializer.end()
    }
}
