pub mod de;
pub mod ser;

use std::collections::{hash_map, HashMap};
use std::ops::{Index, IndexMut};

use crate::arch::{
    component::{Component, ComponentBuilder, ComponentData, ComponentRef},
    port::{PortData, PortId},
    ComponentId, StringId,
};
use crate::{database::Database, stringtable::StringTable};

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub(crate) name: StringId,
    pub(crate) strings: StringTable<StringId>,
    pub(crate) component_db: Database<ComponentData, ComponentId>,
    pub(crate) components: HashMap<StringId, ComponentId>,
    pub(crate) port_db: Database<PortData, PortId>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let mut strings = StringTable::default();
        let name = strings.entry(name);
        let component_db = Database::default();
        let components = HashMap::default();
        let port_db = Database::default();

        Self {
            name,
            strings,
            component_db,
            components,
            port_db,
        }
    }

    pub fn name(&self) -> &str {
        &self.strings[self.name]
    }

    pub fn rename(&mut self, name: &str) {
        self.name = self.strings.entry(name);
    }

    pub fn components(&self) -> ComponentIter {
        ComponentIter {
            module: self,
            iter: self.components.values(),
        }
    }

    pub fn get_component(&self, component: ComponentId) -> Option<&ComponentData> {
        if self.components.values().any(|c| c == &component) {
            Some(&self[component])
        } else {
            None
        }
    }

    pub fn get_component_mut(&mut self, component: ComponentId) -> Option<&mut ComponentData> {
        if self.components.values().any(|c| c == &component) {
            Some(&mut self[component])
        } else {
            None
        }
    }
}

impl Index<ComponentId> for Module {
    type Output = ComponentData;

    fn index(&self, index: ComponentId) -> &Self::Output {
        &self.component_db[index]
    }
}

impl IndexMut<ComponentId> for Module {
    fn index_mut(&mut self, index: ComponentId) -> &mut Self::Output {
        &mut self.component_db[index]
    }
}

impl Index<PortId> for Module {
    type Output = PortData;

    fn index(&self, index: PortId) -> &Self::Output {
        &self.port_db[index]
    }
}

impl IndexMut<PortId> for Module {
    fn index_mut(&mut self, index: PortId) -> &mut Self::Output {
        &mut self.port_db[index]
    }
}

pub struct ComponentIter<'m> {
    module: &'m Module,
    iter: hash_map::Values<'m, StringId, ComponentId>,
}

impl<'m> Iterator for ComponentIter<'m> {
    type Item = Component<'m>;

    fn next(&mut self) -> Option<Self::Item> {
        let component = *self.iter.next()?;
        Some(component.to_component(self.module))
    }
}

pub struct ModuleBuilder {
    module: Module,
    name_is_set: bool,
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub enum ModuleBuildError {
    DuplicateReference {
        component: String,
        reference: String,
    },
    MissingField(&'static str),
    UndefinedReference {
        component: String,
        reference: String,
    },
}

pub trait Resolve {
    fn resolve(
        &self,
        module: &Module,
        component: ComponentId,
    ) -> Result<(StringId, ComponentRef), ModuleBuildError>;
}

impl Resolve for StringId {
    fn resolve(
        &self,
        module: &Module,
        component: ComponentId,
    ) -> Result<(StringId, ComponentRef), ModuleBuildError> {
        if let Some(component) = module.components.get(self) {
            let alias = module.component_db[*component].name;
            Ok((alias, component.reference()))
        } else {
            let reference = module.strings[*self].to_string();
            let component = module[component].name(module).to_string();
            Err(ModuleBuildError::UndefinedReference {
                component,
                reference,
            })
        }
    }
}

impl Resolve for (StringId, StringId) {
    fn resolve(
        &self,
        module: &Module,
        component: ComponentId,
    ) -> Result<(StringId, ComponentRef), ModuleBuildError> {
        if let Some(component) = module.components.get(&self.1) {
            Ok((self.0, component.reference()))
        } else {
            let reference = module.strings[self.1].to_string();
            let component = module[component].name(module).to_string();
            Err(ModuleBuildError::UndefinedReference {
                component,
                reference,
            })
        }
    }
}

impl ModuleBuilder {
    pub fn new() -> Self {
        let module = Module::new("");

        Self {
            module,
            name_is_set: false,
        }
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.module.rename(name);
        self.name_is_set = true;
        self
    }

    pub fn add_component(&mut self) -> ComponentBuilder<'_> {
        ComponentBuilder::new(&mut self.module)
    }

    pub fn resolve_references<I, R>(
        &mut self,
        component: ComponentId,
        references: I,
    ) -> Result<&mut Self, ModuleBuildError>
    where
        I: Iterator<Item = R>,
        R: Resolve,
    {
        let module = &mut self.module;

        for reference in references {
            let (alias, reference) = reference.resolve(module, component)?;
            if module[component]
                .references
                .insert(alias, reference)
                .is_some()
            {
                let component = module.component_db[component].name(module).to_string();
                let reference = module.strings[alias].to_string();
                return Err(ModuleBuildError::DuplicateReference {
                    component,
                    reference,
                });
            }
        }

        Ok(self)
    }

    pub fn is_name_set(&self) -> bool {
        self.name_is_set
    }

    pub fn is_components_empty(&self) -> bool {
        self.module.components.is_empty()
    }

    pub fn finish(self) -> Result<Module, ModuleBuildError> {
        if !self.is_name_set() {
            return Err(ModuleBuildError::MissingField("name"));
        }

        Ok(self.module)
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
