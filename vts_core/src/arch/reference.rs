use std::fmt;

use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

use super::{
    component::ComponentKey,
    linker::{self, Components, Resolve},
    Component, ComponentId, ComponentRefId, Module,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentRefData {
    pub(crate) component: ComponentId,
    pub(crate) alias: Option<String>,
    pub n_instances: usize,
}

impl ComponentRefData {
    pub(crate) fn new(component: ComponentId, alias: Option<String>, n_instances: usize) -> Self {
        Self {
            component,
            alias,
            n_instances,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ComponentRefKey(pub(crate) ComponentRefId);

impl ComponentRefKey {
    pub(crate) fn new(reference: ComponentRefId) -> Self {
        Self(reference)
    }

    pub fn bind(self, module: &Module) -> ComponentRef<'_> {
        ComponentRef::new(module, self.0)
    }
}

#[derive(Clone, Debug)]
pub struct ComponentRef<'m>(&'m Module, ComponentRefId);

impl<'m> ComponentRef<'m> {
    pub(crate) fn new(module: &'m Module, reference: ComponentRefId) -> Self {
        Self(module, reference)
    }

    pub(crate) fn module(&self) -> &'m Module {
        self.0
    }

    pub fn key(&self) -> ComponentRefKey {
        ComponentRefKey::new(self.1)
    }

    pub(crate) fn data(&self) -> &'m ComponentRefData {
        &self.module()[self.1]
    }

    pub fn component(&self) -> Component<'m> {
        Component::new(self.module(), self.data().component)
    }

    pub fn alias(&self) -> Option<&'m String> {
        self.data().alias.as_ref()
    }

    pub fn alias_or_name(&self) -> &'m str {
        if let Some(alias) = self.alias() {
            alias
        } else {
            self.component().name()
        }
    }

    pub fn n_instances(&self) -> usize {
        self.data().n_instances
    }
}

pub struct ComponentSet(ComponentId);
pub struct ComponentUnset;

pub struct ComponentRefBuilder<'m, C> {
    module: &'m mut Module,
    parent: ComponentId,
    component: C,
    alias: Option<String>,
    n_instances: Option<usize>,
}

impl<'m> ComponentRefBuilder<'m, ComponentUnset> {
    pub fn new(module: &'m mut Module, parent: ComponentKey) -> Self {
        Self {
            module,
            parent: parent.0,
            component: ComponentUnset,
            alias: None,
            n_instances: None,
        }
    }

    pub fn set_component(self, component: ComponentKey) -> ComponentRefBuilder<'m, ComponentSet> {
        ComponentRefBuilder {
            module: self.module,
            parent: self.parent,
            component: ComponentSet(component.0),
            alias: self.alias,
            n_instances: self.n_instances,
        }
    }
}

impl<'m, C> ComponentRefBuilder<'m, C> {
    pub fn set_alias(&mut self, alias: &str) {
        self.alias = Some(alias.to_string());
    }

    pub fn set_n_instances(&mut self, n_instances: usize) {
        self.n_instances = Some(n_instances);
    }

    pub fn alias_is_set(&self) -> bool {
        self.alias.is_some()
    }

    pub fn n_instances_is_set(&self) -> bool {
        self.n_instances.is_some()
    }
}

impl<'m> ComponentRefBuilder<'m, ComponentSet> {
    fn insert(&mut self) -> ComponentRefId {
        let n_instances = self.n_instances.unwrap_or(1);
        let reference = ComponentRefData::new(self.component.0, self.alias.take(), n_instances);
        self.module.references.insert(reference)
    }

    pub fn finish(mut self) -> ComponentRef<'m> {
        // TODO: check duplicate references
        let reference = self.insert();
        self.module[self.parent].references.push(reference);
        ComponentRef::new(self.module, reference)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct ComponentWeakRef {
    pub component: String,
    #[serde(skip)]
    pub alias: Option<String>,
    pub n_instances: usize,
}

pub enum DeserializeComponentWeakRef {
    Named(String),
    Unnamed,
}

const FIELDS: &[&str] = &["component", "n_instances"];
const COMPONENT: usize = 0;
const N_INSTANCES: usize = 1;

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
        #[serde(rename_all = "lowercase")]
        enum Field {
            Component,
            #[serde(rename = "n_instances")]
            NInstances,
        }

        let mut component: Option<String> = None;
        let mut n_instances: Option<usize> = None;

        while let Some(field) = map.next_key()? {
            match field {
                Field::Component => {
                    if component.is_some() {
                        return Err(de::Error::duplicate_field(FIELDS[COMPONENT]));
                    }

                    component = Some(map.next_value()?);
                }
                Field::NInstances => {
                    if n_instances.is_some() {
                        return Err(de::Error::duplicate_field(FIELDS[N_INSTANCES]));
                    }

                    n_instances = Some(map.next_value()?);
                }
            }
        }

        let component = component.ok_or(de::Error::missing_field(FIELDS[COMPONENT]))?;
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
        deserializer.deserialize_struct("ComponentRef", FIELDS, self)
    }
}

impl<'m> Resolve<'m> for ComponentWeakRef {
    type Output = ComponentRefKey;

    fn resolve(
        self,
        module: &mut Module,
        component: ComponentKey,
        components: &Components,
    ) -> Result<Self::Output, linker::Error> {
        let referenced_component = {
            let component = self.component.as_str();
            components.get(module, component)?.key()
        };

        let mut builder =
            ComponentRefBuilder::new(module, component).set_component(referenced_component);

        if let Some(alias) = &self.alias {
            builder.set_alias(alias);
        }

        Ok(builder.finish().key())
    }
}
