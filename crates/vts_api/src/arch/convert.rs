use pyo3::prelude::*;
use pyo3::types::PyString;
use thiserror::Error;
use vts_core::arch::{
    component::{ComponentBuildError, ConnectionBuildError},
    module::{ModuleBuildError, ModuleBuilder},
    port::PortBuildError,
    Module,
};

use crate::arch::{PyComponent, PyComponentRef, PyConnection, PyModule as PyModule_, PyPort};

pub trait Converter {
    type Output;
    type Error;

    fn convert(self) -> Result<Self::Output, Self::Error>;
}

pub struct PyModuleConverter<'py>(pub(crate) Bound<'py, PyModule_>);

#[derive(Debug, Error)]
pub enum PyModuleConvertError {
    #[error("{0}")]
    Component(#[from] ComponentBuildError),
    #[error("{0}")]
    Connection(#[from] ConnectionBuildError),
    #[error("{0}")]
    Module(#[from] ModuleBuildError),
    #[error("{0}")]
    Port(#[from] PortBuildError),
    #[error("{0}")]
    Python(#[from] PyErr),
}

impl Converter for PyModuleConverter<'_> {
    type Output = Module;
    type Error = PyModuleConvertError;

    fn convert(self) -> Result<Self::Output, Self::Error> {
        let PyModuleConverter(module) = self;
        let py = module.py();

        let mut module_builder = ModuleBuilder::new();
        let module = module.borrow();

        module_builder.set_name(module.name.to_str(py)?);

        let mut unresolved = Vec::new();

        let components = module.components.bind(py);
        iter_dict_items!(for (name: PyString, component: PyComponent) in components => {
            let mut component_builder = module_builder.add_component();
            let component = component.borrow();

            component_builder.set_name(name.to_str()?);

            let ports = component.ports.bind(py);
            iter_dict_items!(for (name: PyString, port: PyPort) in ports => {
                let mut port_builder = component_builder.add_port();
                let port = port.borrow();

                port_builder.set_name(name.to_str()?);
                port_builder.set_kind(port.kind.into());
                port_builder.set_n_pins(port.n_pins);

                if let Some(class) = port.class_ {
                    port_builder.set_class(class.into());
                }

                port_builder.finish()?;
            });

            let references = component.references.bind(py);
            iter_dict_items!(for (alias: PyString, reference: PyComponentRef) in references => {
                let reference = reference.borrow();
                let n_instances = reference.n_instances;
                let component = reference.component.borrow(py);
                component_builder.add_weak_reference(alias.to_str()?, Some(component.name.to_str(py)?), Some(n_instances))?;
            });

            let connections = component.connections.bind(py);
            iter_list_items!(for (connection: PyConnection) in connections => {
                let mut connection_builder = component_builder.add_weak_connection();
                let connection = connection.borrow();

                let source_pins = connection.source_pins.borrow(py);
                let source_port = source_pins.port.borrow(py);
                let source_port = source_port.name.bind(py);
                let alias = if let Some(ref component) = connection.source_component {
                    let component = component.borrow(py);
                    if let Some(ref alias) = component.alias {
                        Some(alias.to_str(py)?.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                connection_builder.set_source(source_port.to_str()?, source_pins.range.clone(), alias.as_ref().map(|alias| alias.as_str()));

                let sink_pins = connection.sink_pins.borrow(py);
                let sink_port = sink_pins.port.borrow(py);
                let sink_port = sink_port.name.bind(py);
                let alias = if let Some(ref component) = connection.source_component {
                    let component = component.borrow(py);
                    if let Some(ref alias) = component.alias {
                        Some(alias.to_str(py)?.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                connection_builder.set_sink(sink_port.to_str()?, sink_pins.range.clone(), alias.as_ref().map(|alias| alias.as_str()));

                connection_builder.finish()?;
            });

            if let Some(class) = component.class_ {
                component_builder.set_class(class.into());
            }

            unresolved.push(component_builder.finish()?);
        });

        for (component, references, named_references) in unresolved {
            module_builder.resolve_references(component, references.into_iter())?;
            module_builder.resolve_references(component, named_references.into_iter())?;
        }

        module_builder.resolve_connections();

        Ok(module_builder.finish()?)
    }
}

pub struct ModuleConverter<'py>(pub(crate) Python<'py>, pub(crate) Module);

impl Converter for ModuleConverter<'_> {
    type Output = Py<PyModule_>;
    type Error = PyErr;

    fn convert(self) -> Result<Self::Output, Self::Error> {
        let ModuleConverter(py, module) = self;

        let name = PyString::new_bound(py, module.name());
        let mut py_module = PyModule_::new(&name);

        for component in module.components() {
            let name = PyString::new_bound(py, component.name());
            let class = component.class().map(Into::into);
            let mut py_component = PyComponent::new(&name, class);

            for port in component.ports() {
                let name = PyString::new_bound(py, port.name());
                let kind = port.kind().into();
                let class = port.class().map(Into::into);
                let py_port = PyPort::new(&name, kind, Some(port.n_pins()), class);
                let py_port = Bound::new(py, py_port)?;
                py_component.add_port(&name, &py_port)?;
            }

            let py_component = Bound::new(py, py_component)?;
            py_module.add_component(&name, &py_component)?;
            let components = py_module.components.bind(py);
            assert!(components.contains(name)?);
        }

        let components = py_module.components.bind(py);
        for component in module.components() {
            let component_name = PyString::new_bound(py, component.name());

            let py_component = get_dict_item!(components, component_name as PyComponent)
                .expect("component should be in module");

            let mut py_component = py_component.borrow_mut();

            for (alias, reference) in component.references() {
                let alias = PyString::new_bound(py, alias);
                let reference_name = PyString::new_bound(py, reference.component().name());

                let py_reference = get_dict_item!(components, reference_name as PyComponent)
                    .expect("referenced component should be in module");

                py_component.add_reference(
                    &py_reference,
                    Some(&alias),
                    Some(reference.n_instances()),
                )?;
            }
        }

        // TODO: support connections

        Py::new(py, py_module)
    }
}
