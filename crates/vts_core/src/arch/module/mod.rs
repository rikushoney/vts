pub mod de;
pub mod ser;

use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use crate::arch::{
    component::{ComponentBuilder, ComponentData, ComponentRef},
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
            let alias = module.component_db.lookup(*component).name;
            Ok((alias, component.reference()))
        } else {
            let reference = module.strings.lookup(*self).to_string();
            let component = module.component(component).name(module).to_string();
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
            let reference = module.strings.lookup(self.1).to_string();
            let component = module.component(component).name(module).to_string();
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
            if module
                .component_mut(component)
                .references
                .insert(alias, reference)
                .is_some()
            {
                let component = module
                    .component_db
                    .lookup(component)
                    .name(module)
                    .to_string();
                let reference = module.strings.lookup(alias).to_string();
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
