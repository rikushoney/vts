use std::collections::HashMap;

use serde::Deserialize;
use vts_shared::{
    database::{Database, DbKey},
    stringtable::{StringTable, TableKey},
    OpaqueKey,
};

// TODO: make this a derive macro in vts_shared
macro_rules! impl_opaquekey_wrapper {
    ($name:ident, $base:path) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        struct $name($base);

        impl OpaqueKey for $name {
            fn as_index(&self) -> usize {
                self.0.as_index()
            }

            fn from_index(idx: usize) -> Self {
                $name(<$base as OpaqueKey>::from_index(idx))
            }

            fn max_index() -> usize {
                <$base as OpaqueKey>::max_index()
            }
        }
    };
}

macro_rules! impl_dbkey_wrapper {
    ($name:ident, $base:path) => {
        impl_opaquekey_wrapper!($name, $base);

        impl DbKey for $name {}
    };
}

impl_dbkey_wrapper!(ComponentId, u32);
impl_dbkey_wrapper!(PortId, u32);

impl_opaquekey_wrapper!(StringId, u32);

impl TableKey for StringId {}

#[derive(Clone, Debug, PartialEq)]
pub struct Module<'m> {
    name: StringId,
    strings: StringTable<StringId>,
    components: Database<Component<'m>, ComponentId>,
    component_name_map: HashMap<StringId, ComponentId>,
    ports: Database<Port<'m>, PortId>,
    port_name_map: HashMap<StringId, PortId>,
}

impl<'m> Module<'m> {
    pub fn new(name: &str) -> Self {
        let mut strings = StringTable::default();
        let name = strings.entry(name);
        let components = Database::default();
        let component_name_map = HashMap::default();
        let ports = Database::default();
        let port_name_map = HashMap::default();

        Self {
            name,
            strings,
            components,
            component_name_map,
            ports,
            port_name_map,
        }
    }

    pub fn name(&self) -> &str {
        self.strings.lookup(self.name)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component<'m> {
    module: &'m Module<'m>,
    name: StringId,
    ports: HashMap<StringId, PortId>,
    references: HashMap<StringId, ComponentId>,
    class: Option<ComponentClass>,
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
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PortKind {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum PortClass {
    #[serde(rename = "CLOCK")]
    Clock,
    #[serde(rename = "LUT_IN")]
    LutIn,
    #[serde(rename = "LUT_OUT")]
    LutOut,
    #[serde(rename = "LATCH_IN")]
    LatchIn,
    #[serde(rename = "LATCH_OUT")]
    LatchOut,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port<'m> {
    module: &'m Module<'m>,
    name: StringId,
    kind: PortKind,
    n_pins: usize,
    class: Option<PortClass>,
}

impl<'m> Port<'m> {
    pub fn new(
        module: &'m mut Module,
        name: &str,
        kind: PortKind,
        n_pins: usize,
        class: Option<PortClass>,
    ) -> Self {
        let name = module.strings.entry(name);

        Self {
            module,
            name,
            kind,
            n_pins,
            class,
        }
    }

    pub fn name(&self) -> &str {
        self.module.strings.lookup(self.name)
    }
}
