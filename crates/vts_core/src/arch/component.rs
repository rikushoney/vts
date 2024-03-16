use std::collections::{HashMap, HashSet};

use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Deserialize, Serialize, Serializer,
};

use super::{
    impl_dbkey_wrapper,
    port::{PortId, PortRecipe, PortSerializer},
    Module, Port, StringId,
};

impl_dbkey_wrapper!(ComponentId, u32);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComponentClass {
    Lut,
    Latch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub(crate) name: StringId,
    pub(crate) ports: HashMap<StringId, PortId>,
    references: HashMap<StringId, ComponentId>,
    pub class: Option<ComponentClass>,
}

impl Component {
    pub(crate) fn new(module: &mut Module, name: &str, class: Option<ComponentClass>) -> Component {
        let name = module.strings.entry(name);
        assert!(module.component_names.get(&name).is_none(), "{}", {
            let name = module.strings.lookup(name);
            let module_name = module.strings.lookup(module.name);
            format!(r#"component "{name}" already in module "{module_name}""#)
        });

        let ports = HashMap::default();
        let references = HashMap::default();

        Self {
            name,
            ports,
            references,
            class,
        }
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.lookup(self.name)
    }

    pub fn port<'m>(&self, module: &'m Module, port: PortId) -> &'m Port {
        assert!(self.ports.values().any(|p| p == &port), "{}", {
            let port_name = module.ports.lookup(port).name(module);
            let component_name = self.name(module);
            format!(r#"port "{port_name}" is not in component "{component_name}""#)
        });
        module.ports.lookup(port)
    }

    pub fn port_mut<'m>(&'m self, module: &'m mut Module, port: PortId) -> &'m mut Port {
        assert!(self.ports.values().any(|p| p == &port), "{}", {
            let port_name = module.ports.lookup(port).name(module);
            let component_name = self.name(module);
            format!(r#"port "{port_name}" is not in component "{component_name}""#)
        });
        module.ports.lookup_mut(port)
    }

    pub fn port_id(&self, module: &Module, name: &str) -> Option<PortId> {
        let name = module.strings.rlookup(name)?;
        self.ports.get(&name).copied()
    }

    pub fn add_port<'m>(&'m mut self, module: &'m mut Module, recipe: &PortRecipe) -> PortId {
        let port = recipe.instantiate(module, self);

        debug_assert!(self.ports.values().any(|p| p == &port));
        debug_assert!({
            let name = module
                .strings
                .rlookup(self.port(module, port).name(module))
                .expect("port should be instantiated");
            self.ports.contains_key(&name)
        });

        port
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ComponentRecipe {
    #[serde(skip_deserializing)]
    pub(crate) name: Option<String>,
    ports: Option<HashMap<String, PortRecipe>>,
    references: Option<HashSet<String>>,
    class: Option<ComponentClass>,
}

impl ComponentRecipe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn port(&mut self, recipe: &PortRecipe) -> &mut Self {
        if let Some(ref mut ports) = self.ports {
            let name = recipe
                .name
                .as_ref()
                .expect("port should have a name")
                .clone();
            assert!(ports.insert(name, recipe.clone()).is_none(), "{}", {
                let port_name = recipe.name.as_ref().unwrap();
                let component_name = match self.name {
                    Some(ref name) => name.clone(),
                    None => String::new(),
                };
                format!(r#"port "{port_name}" already in component "{component_name}""#)
            });
            self
        } else {
            self.ports = Some(HashMap::default());
            self.port(recipe)
        }
    }

    pub fn ports<'a, I: Iterator<Item = &'a PortRecipe>>(&mut self, recipes: I) -> &mut Self {
        for recipe in recipes {
            self.port(recipe);
        }
        self
    }

    pub fn reference(&mut self, reference: &str) -> &mut Self {
        if let Some(ref mut references) = self.references {
            assert!(references.insert(reference.to_string()), "{}", {
                let component_name = match self.name {
                    Some(ref name) => name.clone(),
                    None => String::new(),
                };
                format!(
                    r#"component "{reference}" already referenced in component "{component_name}""#
                )
            });
            self
        } else {
            self.references = Some(HashSet::default());
            self.reference(reference)
        }
    }

    pub fn references<'a, I: Iterator<Item = &'a str>>(&mut self, references: I) -> &mut Self {
        for reference in references {
            self.reference(reference);
        }
        self
    }

    pub fn class(&mut self, class: ComponentClass) -> &mut Self {
        self.class = Some(class);
        self
    }

    pub(crate) fn instantiate(&self, module: &mut Module) -> ComponentId {
        let mut component = Component::new(
            module,
            self.name
                .as_ref()
                .expect("component should have a name")
                .as_str(),
            self.class,
        );

        if let Some(ref ports) = self.ports {
            for port in ports.values() {
                component.add_port(module, port);
            }
        }

        // TODO: references

        let name = component.name;
        let component = module.components.entry(component);

        assert!(
            module.component_names.insert(name, component).is_none(),
            "{}",
            {
                let component_name = module.strings.lookup(name);
                let module_name = module.strings.lookup(name);
                format!(r#"component "{component_name}" already in module "{module_name}""#)
            }
        );

        component
    }
}

struct PortsSerializer<'a, 'm> {
    module: &'m Module,
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
            serializer.serialize_entry(
                name,
                &PortSerializer {
                    _module: self.module,
                    port,
                },
            )?;
        }

        serializer.end()
    }
}

struct ComponentRefsSerializer<'a, 'm> {
    module: &'m Module,
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

pub(crate) struct ComponentSerializer<'m> {
    pub(crate) module: &'m Module,
    pub(crate) component: &'m Component,
}

impl<'m> Serialize for ComponentSerializer<'m> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Component", 4)?;

        let ports_serializer = PortsSerializer {
            module: self.module,
            ports: &self.component.ports,
        };
        serializer.serialize_field("ports", &ports_serializer)?;

        let component_refs_serializer = ComponentRefsSerializer {
            module: self.module,
            references: &self.component.references,
        };
        serializer.serialize_field("references", &component_refs_serializer)?;

        serializer.serialize_field("class", &self.component.class)?;

        serializer.end()
    }
}
