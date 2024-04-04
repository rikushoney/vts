use pyo3::prelude::*;
use pyo3::types::PyString;
use thiserror::Error;
use vts_core::arch::{
    component::{ComponentBuildError, ConnectionBuildError},
    module::{ModuleBuildError, ModuleBuilder},
    port::PortBuildError,
    Module,
};

use crate::arch::{
    PyComponent, PyComponentRef, PyConnection, PyModule as PyModule_, PyPort, PyPortPins,
};

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
                component_builder.add_weak_reference(component.name.to_str(py)?, Some(alias.to_str()?), Some(n_instances))?;
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
                connection_builder.set_source(source_port.to_str()?, source_pins.range.clone(), alias.as_deref());

                let sink_pins = connection.sink_pins.borrow(py);
                let sink_port = sink_pins.port.borrow(py);
                let sink_port = sink_port.name.bind(py);
                let alias = if let Some(ref component) = connection.sink_component {
                    let component = component.borrow(py);
                    if let Some(ref alias) = component.alias {
                        Some(alias.to_str(py)?.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                connection_builder.set_sink(sink_port.to_str()?, sink_pins.range.clone(), alias.as_deref());

                connection_builder.finish()?;
            });

            if let Some(class) = component.class_ {
                component_builder.set_class(class.into());
            }

            unresolved.push(component_builder.finish()?);
        });

        module_builder
            .resolve_and_finish(unresolved)
            .map_err(PyModuleConvertError::from)
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
            debug_assert!(components.contains(name)?);
        }

        let components = py_module.components.bind(py);
        for component in module.components() {
            let component_name = PyString::new_bound(py, component.name());

            let py_component = get_dict_item!(components, component_name as PyComponent)
                .expect("component should be in module");

            for (alias, reference) in component.references() {
                let alias = PyString::new_bound(py, alias);
                let reference_name = PyString::new_bound(py, reference.component().name());

                let py_reference = get_dict_item!(components, reference_name as PyComponent)
                    .expect("referenced component should be in module");

                py_component.borrow_mut().add_reference(
                    &py_reference,
                    Some(&alias),
                    Some(reference.n_instances()),
                )?;
            }

            for connection in component.connections() {
                let source_reference = connection.source_component(&module);
                let sink_reference = connection.sink_component(&module);

                let source_pins = connection.source_pins();
                let source_start = source_pins.start();
                let source_end = source_pins.end();

                let sink_pins = connection.sink_pins();
                let sink_start = sink_pins.start();
                let sink_end = sink_pins.end();

                let source_port = connection
                    .source_port(&module, &component)
                    .expect("source port should be in component");

                let sink_port = connection
                    .sink_port(&module, &component)
                    .expect("sink port should be in component");

                let source_reference = if let Some(source_reference) = source_reference {
                    let py_component = py_component.borrow();
                    let references = py_component.references.bind(py);
                    let reference = references
                        .get_item(source_reference.alias())?
                        .expect("reference should be in component");

                    Some(reference.downcast::<PyComponentRef>()?.clone())
                } else {
                    None
                };

                let sink_reference = if let Some(sink_reference) = sink_reference {
                    let py_component = py_component.borrow();
                    let references = py_component.references.bind(py);
                    let reference = references
                        .get_item(sink_reference.alias())?
                        .expect("reference should be in component");

                    Some(reference.downcast::<PyComponentRef>()?.clone())
                } else {
                    None
                };

                let source_component = if let Some(ref source_reference) = source_reference {
                    source_reference.borrow().component.clone()
                } else {
                    py_component.clone().unbind()
                };

                let sink_component = if let Some(ref sink_reference) = sink_reference {
                    sink_reference.borrow().component.clone()
                } else {
                    py_component.clone().unbind()
                };

                let source_port = {
                    let source_component = source_component.bind(py).borrow();
                    let ports = source_component.ports.bind(py);
                    let source_port = ports
                        .get_item(source_port.name())?
                        .expect("port should be in component");
                    source_port.downcast::<PyPort>()?.clone()
                };

                let sink_port = {
                    let sink_component = sink_component.bind(py).borrow();
                    let ports = sink_component.ports.bind(py);
                    let sink_port = ports
                        .get_item(sink_port.name())?
                        .expect("port should be in component");
                    sink_port.downcast::<PyPort>()?.clone()
                };

                let source_pins =
                    PyPortPins::new(source_port, Some(source_start), Some(source_end));
                let source_pins = Bound::new(py, source_pins)?;

                let sink_pins = PyPortPins::new(sink_port, Some(sink_start), Some(sink_end));
                let sink_pins = Bound::new(py, sink_pins)?;

                let mut py_component = py_component.borrow_mut();
                py_component.add_connection(
                    &source_pins,
                    &sink_pins,
                    source_reference.as_ref(),
                    sink_reference.as_ref(),
                )?;
            }
        }

        Py::new(py, py_module)
    }
}
