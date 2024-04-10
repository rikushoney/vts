use std::collections::HashMap;

use fnv::FnvHashMap;
use thiserror::Error;

use super::component::ComponentKey;
use super::module::{ComponentId, ComponentRefId, Module};
use super::reference::{ComponentRefBuilder, ComponentRefKey, ComponentWeakRef};

#[derive(Debug, Error)]
pub enum LinkerError {
    #[error(r#"undefined component "{component}" referenced in "{module}""#)]
    UndefinedComponent { module: String, component: String },
}

#[derive(Default)]
pub struct Linker {
    unresolved_references: HashMap<ComponentId, ComponentWeakRef>,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            unresolved_references: HashMap::default(),
        }
    }

    pub fn add_reference(&mut self, component: ComponentKey, reference: ComponentWeakRef) {
        // TODO: check duplicate references
        self.unresolved_references.insert(component.0, reference);
    }

    fn resolve_reference_impl(
        &self,
        module: &mut Module,
        component: ComponentId,
        reference: &ComponentWeakRef,
        hint: Option<ComponentId>,
    ) -> Result<ComponentRefId, LinkerError> {
        let referenced_component = if let Some(component) = hint {
            ComponentKey::new(component)
        } else {
            module
                .find_component(&reference.component)
                .ok_or(LinkerError::UndefinedComponent {
                    module: module.name().to_string(),
                    component: reference.component.clone(),
                })?
                .key()
        };

        let mut builder = ComponentRefBuilder::new(module, ComponentKey::new(component))
            .set_component(referenced_component);

        builder.set_n_instances(reference.n_instances);
        Ok(builder.finish().key().0)
    }

    pub fn resolve_reference(
        &self,
        module: &mut Module,
        component: ComponentKey,
        reference: &ComponentWeakRef,
        hint: Option<ComponentKey>,
    ) -> Result<ComponentRefKey, LinkerError> {
        self.resolve_reference_impl(module, component.0, reference, hint.map(|hint| hint.0))
            .map(ComponentRefKey::new)
    }

    pub fn resolve_references(&self, module: &mut Module) -> Result<(), LinkerError> {
        let mut cached_components = FnvHashMap::<&str, ComponentId>::default();

        for (&component, reference) in self.unresolved_references.iter() {
            let component_name = reference.component.as_str();

            match cached_components.get(component_name) {
                Some(&hint) => {
                    self.resolve_reference_impl(module, component, reference, Some(hint))?;
                }
                None => {
                    let reference =
                        self.resolve_reference_impl(module, component, reference, None)?;

                    let component = module[reference].component;
                    cached_components.insert(component_name, component);
                }
            }
        }

        Ok(())
    }
}
