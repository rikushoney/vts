use std::fmt;
use std::ops::{Index, IndexMut};

use serde::{
    de::{self, DeserializeSeed, MapAccess, Unexpected, Visitor},
    ser::{SerializeMap, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};
use slotmap::{new_key_type, SlotMap};

use super::component::{Component, ComponentData, ComponentKey, ComponentSeed, SerializeComponent};
use super::linker::Linker;
use super::port::{Port, PortData, PortKey};
use super::reference::{ComponentRef, ComponentRefData, ComponentRefKey};

new_key_type! {
    pub(crate) struct ComponentId;
    pub(crate) struct ComponentRefId;
    pub(crate) struct PortId;
}

#[derive(Clone, Debug)]
pub struct Module {
    pub(crate) name: String,
    pub(crate) components: SlotMap<ComponentId, ComponentData>,
    pub(crate) ports: SlotMap<PortId, PortData>,
    pub(crate) references: SlotMap<ComponentRefId, ComponentRefData>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            components: SlotMap::default(),
            ports: SlotMap::default(),
            references: SlotMap::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rename(&mut self, name: &str) {
        self.name = name.to_string();
    }

    // pub fn components(&self) -> ComponentIter {
    //     ComponentIter {
    //         module: self,
    //         iter: self.component_name_map.values(),
    //     }
    // }

    pub fn get_component(&self, component: ComponentKey) -> Option<Component<'_>> {
        let component = component.0;

        if self.components.contains_key(component) {
            Some(Component::new(self, component))
        } else {
            None
        }
    }

    // pub fn get_component_mut(&mut self, component: ComponentId) -> Option<&mut ComponentData> {
    //     if self.components.values().any(|c| c == &component) {
    //         Some(&mut self[component])
    //     } else {
    //         None
    //     }
    // }

    pub fn get_port(&self, port: PortKey) -> Option<Port<'_>> {
        let port = port.0;

        if self.ports.contains_key(port) {
            Some(Port::new(self, port))
        } else {
            None
        }
    }

    pub fn get_reference(&self, reference: ComponentRefKey) -> Option<ComponentRef<'_>> {
        let reference = reference.0;

        if self.references.contains_key(reference) {
            Some(ComponentRef::new(self, reference))
        } else {
            None
        }
    }

    pub fn find_component(&self, name: &str) -> Option<Component<'_>> {
        self.components
            .iter()
            .find(|(_, component)| component.name == name)
            .map(|(component, _)| Component::new(self, component))
    }
}

macro_rules! impl_module_index_ops {
    ($($id:ident => $data:ident in $db:ident),+ $(,)?) => {
        $(
            impl Index<$id> for Module {
                type Output = $data;

                fn index(&self, id: $id) -> &Self::Output {
                    &self.$db[id]
                }
            }

            impl IndexMut<$id> for Module {
                fn index_mut(&mut self, id: $id) -> &mut Self::Output {
                    &mut self.$db[id]
                }
            }
        )+
    }
}

impl_module_index_ops!(
    ComponentId => ComponentData in components,
    PortId => PortData in ports,
    ComponentRefId => ComponentRefData in references,
);

// pub struct ComponentIter<'m> {
//     module: &'m Module,
//     iter: hash_map::Values<'m, String, ComponentId>,
// }

// impl<'m> Iterator for ComponentIter<'m> {
//     type Item = Component<'m>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let component = *self.iter.next()?;
//         Some(component.to_component(self.module))
//     }
// }

const FIELDS: &[&str] = &["name", "components"];

const NAME: usize = 0;
const COMPONENTS: usize = 1;

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
        let mut state = serializer.serialize_struct("Module", FIELDS.len())?;
        state.serialize_field(FIELDS[NAME], self.name())?;

        state.serialize_field(
            FIELDS[COMPONENTS],
            &SerializeComponents {
                module: self,
                components: &self.components,
            },
        )?;

        state.end()
    }
}

struct DeserializeComponents<'a, 'm> {
    module: &'m mut Module,
    linker: &'a mut Linker,
}

impl<'a, 'de, 'm> Visitor<'de> for DeserializeComponents<'a, 'm> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a dict of components")
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
                #[serde(rename_all = "lowercase")]
                enum Field {
                    Name,
                    Components,
                }

                let mut module = Module::new("__vts_placeholder_module_name");
                let mut linker = Linker::new();
                let mut name = false;

                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Name => {
                            if name {
                                return Err(de::Error::duplicate_field("name"));
                            }

                            module.rename(map.next_value()?);
                            name = true;
                        }
                        Field::Components => {
                            if !module.components.is_empty() {
                                return Err(de::Error::duplicate_field("components"));
                            }

                            map.next_value_seed(DeserializeComponents {
                                module: &mut module,
                                linker: &mut linker,
                            })?;
                        }
                    }
                }

                if !name {
                    return Err(de::Error::missing_field("name"));
                }

                linker
                    .resolve(&mut module)
                    .map_err(|err| de::Error::custom(format!("{err}")))?;

                Ok(module)
            }
        }

        deserializer.deserialize_struct("Module", FIELDS, ModuleVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module() {
        let mut _module = Module::new("test_mod");
    }
}
