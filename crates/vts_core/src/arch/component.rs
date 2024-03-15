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
    pub(crate) fn new(
        module: &mut Module,
        name: &str,
        class: Option<ComponentClass>,
    ) -> ComponentId {
        let name = module.strings.borrow_mut().entry(name);
        if let Some(_) = module.component_name_map.borrow().get(&name) {
            let name = module.strings.borrow().lookup(name);
            let module_name = module.strings.borrow().lookup(module.name);
            panic!(r#"component "{name}" already in module "{module_name}""#)
        }
        let ports = HashMap::default();
        let references = HashMap::default();

        let component = module.components.borrow_mut().entry(Self {
            name,
            ports,
            references,
            class,
        });

        {
            let mut component_name_map = module.component_name_map.borrow_mut();
            component_name_map.insert(name, component);
        }

        component
    }

    pub fn name<'m>(&'m self, module: &'m Module) -> &str {
        module.strings.borrow().lookup(self.name)
    }

    pub fn port<'m>(&'m self, module: &'m Module, name: &str) -> Option<&Port> {
        let name = module.strings.borrow().rlookup(name)?;
        let id = *self.ports.get(&name)?;

        Some(module.ports.borrow().lookup(id))
    }

    pub fn add_port<'m>(&'m mut self, module: &'m mut Module, recipe: &PortRecipe) -> &Port {
        recipe.instantiaite(module, self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRecipe {
    pub(crate) name: Option<String>,
    ports: Option<HashMap<String, PortRecipe>>,
    references: Option<HashSet<String>>,
    class: Option<ComponentClass>,
}

impl ComponentRecipe {
    pub fn new() -> Self {
        Self {
            name: None,
            ports: None,
            references: None,
            class: None,
        }
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn port(&mut self, recipe: &PortRecipe) -> &mut Self {
        if let Some(ref mut ports) = self.ports {
            let name = recipe.name.as_ref().expect("port must have a name").clone();
            if ports.insert(name, recipe.clone()).is_none() {
                self
            } else {
                let port_name = recipe.name.as_ref().unwrap();
                let component_name = match self.name {
                    Some(ref name) => name.clone(),
                    None => String::new(),
                };
                panic!(r#"port "{port_name}" already in "{component_name}""#)
            }
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
            if references.insert(reference.to_string()) {
                self
            } else {
                let component_name = match self.name {
                    Some(ref name) => name.clone(),
                    None => String::new(),
                };
                panic!(r#"component "{reference}" already referenced in "{component_name}""#)
            }
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

    pub fn instantiate<'m>(&self, module: &'m mut Module) -> &'m Component {
        let component = Component::new(
            module,
            self.name
                .as_ref()
                .expect("component must have a name")
                .as_str(),
            self.class,
        );
        module.components.borrow().lookup(component)
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
            let name = self.module.strings.borrow().lookup(*name);
            let port = self.module.ports.borrow().lookup(*port);
            serializer.serialize_entry(
                name,
                &PortSerializer {
                    module: self.module,
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
            let name = self.module.strings.borrow().lookup(*name);
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

        let name = self.module.strings.borrow().lookup(self.component.name);
        serializer.serialize_field("name", name)?;

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
