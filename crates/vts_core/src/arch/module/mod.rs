pub mod de;
pub mod ser;

use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use crate::arch::{
    component::{ComponentBuilder, ComponentData},
    port::PortData,
    port::PortId,
    ComponentId, StringId,
};
use crate::{database::Database, stringtable::StringTable, OpaqueKey};

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
        self.strings.lookup(self.name)
    }

    pub fn rename(&mut self, name: &str) {
        self.name = self.strings.entry(name);
    }

    pub fn component(&self, component: ComponentId) -> &ComponentData {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.get_data(component)
    }

    pub fn component_mut(&mut self, component: ComponentId) -> &mut ComponentData {
        assert!(
            self.components.values().any(|c| c == &component),
            r#"component with id "{id}" not in module "{module}""#,
            id = component.as_index(),
            module = self.name()
        );
        self.get_data_mut(component)
    }

    pub fn get_data<T: DataId>(&self, id: T) -> &T::Data {
        T::get_data(self, id)
    }

    pub fn get_data_mut<T: DataId>(&mut self, id: T) -> &mut T::Data {
        T::get_data_mut(self, id)
    }
}

pub trait DataId {
    type Data;

    fn get_data(module: &Module, id: Self) -> &Self::Data;

    fn get_data_mut(module: &mut Module, id: Self) -> &mut Self::Data;
}

impl DataId for PortId {
    type Data = PortData;

    fn get_data(module: &Module, id: Self) -> &Self::Data {
        module.port_db.lookup(id)
    }

    fn get_data_mut(module: &mut Module, id: Self) -> &mut Self::Data {
        module.port_db.lookup_mut(id)
    }
}

impl DataId for ComponentId {
    type Data = ComponentData;

    fn get_data(module: &Module, id: Self) -> &Self::Data {
        module.component_db.lookup(id)
    }

    fn get_data_mut(module: &mut Module, id: Self) -> &mut Self::Data {
        module.component_db.lookup_mut(id)
    }
}

impl<I: DataId> Index<I> for Module {
    type Output = I::Data;

    fn index(&self, id: I) -> &Self::Output {
        I::get_data(self, id)
    }
}

impl<I: DataId> IndexMut<I> for Module {
    fn index_mut(&mut self, id: I) -> &mut Self::Output {
        I::get_data_mut(self, id)
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

impl ModuleBuilder {
    pub fn new() -> Self {
        let module = Module::new("");

        Self {
            module,
            name_is_set: false,
        }
    }

    pub fn name(&mut self, name: &str) -> &mut Self {
        self.module.rename(name);
        self.name_is_set = true;
        self
    }

    pub fn component(&mut self) -> ComponentBuilder<'_> {
        ComponentBuilder::new(&mut self.module)
    }

    pub fn resolve_references<I: Iterator<Item = StringId>>(
        &mut self,
        component: ComponentId,
        references: I,
    ) -> Result<&mut Self, ModuleBuildError> {
        let mut resolved = HashMap::with_capacity(references.size_hint().0);

        let module = &mut self.module;
        for name in references {
            if let Some(reference) = module.components.get(&name) {
                if resolved.insert(name, reference.reference()).is_some() {
                    let reference = module.strings.lookup(name).to_string();
                    let component = module.component(component).name(module).to_string();
                    return Err(ModuleBuildError::DuplicateReference {
                        component,
                        reference,
                    });
                }
            } else {
                let reference = module.strings.lookup(name).to_string();
                let component = module.component(component).name(module).to_string();
                return Err(ModuleBuildError::UndefinedReference {
                    component,
                    reference,
                });
            }
        }

        module.component_mut(component).references.extend(resolved);

        Ok(self)
    }

    pub fn has_name(&self) -> bool {
        self.name_is_set
    }

    pub fn has_components(&self) -> bool {
        !self.module.components.is_empty()
    }

    pub fn finish(self) -> Result<Module, ModuleBuildError> {
        if !self.has_name() {
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
