use pyo3::prelude::*;
use pyo3::types::PyString;
use vts_core::arch::{
    component::ComponentBuildError,
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

macro_rules! impl_convert_error {
    ($name:ident: $($variant:ident ( $wrapped:path ) ),+$(,)?) => {
        pub enum $name {
            $(
                $variant($wrapped),
            )+
        }

        $(
            impl From<$wrapped> for $name {
                fn from(error: $wrapped) -> Self {
                    Self::$variant(error)
                }
            }
        )+
    };
}

pub struct PyModuleConverter<'py>(Bound<'py, PyModule_>);

impl_convert_error!(
    PyModuleConvertError:
        Component(ComponentBuildError),
        Module(ModuleBuildError),
        Port(PortBuildError),
        Python(PyErr)
);

impl Converter for PyModuleConverter<'_> {
    type Output = Module;
    type Error = PyModuleConvertError;

    fn convert(self) -> Result<Self::Output, Self::Error> {
        let PyModuleConverter(module) = self;
        let py = module.py();

        let mut builder = ModuleBuilder::new();
        let module = module.borrow();

        builder.set_name(module.name.to_str(py)?);

        let mut unresolved = Vec::new();

        let components = module.components.bind(py);
        iter_dict_items!(for (name: PyString, component: PyComponent) in components => {
            let mut builder = builder.add_component();
            let component = component.borrow();

            builder.set_name(name.to_str()?);

            let ports = component.ports.bind(py);
            iter_dict_items!(for (name: PyString, port: PyPort) in ports => {
                let mut builder = builder.add_port();
                let port = port.borrow();

                builder.set_name(name.to_str()?);
                builder.set_kind(port.kind.into());
                builder.set_n_pins(port.n_pins);

                if let Some(class) = port.class_ {
                    builder.set_class(class.into());
                }

                builder.finish()?;
            });

            let references = component.references.bind(py);
            iter_dict_items!(for (alias: PyString, reference: PyComponentRef) in references => {
                let reference = reference.borrow();
                let component = reference.component.borrow(py);
                builder.add_named_reference(alias.to_str()?, Some(component.name.to_str(py)?))?;
            });

            let connections = component.connections.bind(py);
            iter_list_items!(for (_connection: PyConnection) in connections => {
                // TODO: support connections
                todo!()
            });

            if let Some(class) = component.class_ {
                builder.set_class(class.into());
            }

            unresolved.push(builder.finish()?);
        });

        for (component, references, named_references) in unresolved {
            builder.resolve_references(component, references.into_iter())?;
            builder.resolve_references(component, named_references.into_iter())?;
        }

        Ok(builder.finish()?)
    }
}

pub struct ModuleConverter<'py>(Python<'py>, Module);

impl_convert_error!(
    ModuleConvertError:
        Python(PyErr)
);

impl Converter for ModuleConverter<'_> {
    type Output = Py<PyModule_>;
    type Error = ModuleConvertError;

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

            // TODO: support connections

            let py_component = Bound::new(py, py_component)?;
            py_module.add_component(&name, &py_component)?;
        }

        let components = py_module.components.bind(py);
        for component in module.components() {
            let component_name = PyString::new_bound(py, component.name());

            let py_component = get_dict_item!(components, component_name as PyComponent)
                .expect("component should be in module");

            let mut py_component = py_component.borrow_mut();

            for (alias, reference) in component.references() {
                let alias = PyString::new_bound(py, alias);
                let reference_name =
                    PyString::new_bound(py, reference.to_component(&module).name());

                let py_reference = get_dict_item!(components, reference_name as PyComponent)
                    .expect("component should be in module");

                py_component.add_reference(&py_reference, Some(&alias))?;
            }
        }

        Ok(Py::new(py, py_module)?)
    }
}
