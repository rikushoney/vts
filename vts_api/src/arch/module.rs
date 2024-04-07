#![allow(unused)] // TODO: remove this!

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_core::arch::{component::ComponentBuilder, ComponentClass, Module};

use super::{PyComponent, PyComponentClass};

#[pyclass]
#[pyo3(name = "PyModule")]
pub struct PyModule_ {
    inner: Module,
}

#[pymethods]
impl PyModule_ {
    #[new]
    pub fn new(name: &Bound<PyString>) -> PyResult<Self> {
        Ok(Self {
            inner: Module::new(name.to_str()?),
        })
    }

    pub fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    pub fn add_component_impl_(
        slf: &Bound<'_, PyModule_>,
        py: Python<'_>,
        name: &Bound<'_, PyString>,
        class: Option<PyComponentClass>,
    ) -> PyResult<Py<PyComponent>> {
        let mut module = slf.borrow_mut();

        // TODO: check for duplicate component names

        let mut builder = ComponentBuilder::new(&mut module.inner).set_name(name.to_str()?);

        if let Some(class) = class {
            builder.set_class(ComponentClass::from(class));
        }

        let component = {
            let component = builder.finish();
            PyComponent(slf.clone().unbind(), component.1)
        };

        Py::new(py, component)
    }

    pub fn add_component_copy_(
        slf: &Bound<'_, PyModule_>,
        py: Python<'_>,
        component: &Bound<'_, PyComponent>,
        name: Option<&Bound<'_, PyString>>,
        class: Option<PyComponentClass>,
    ) -> PyResult<Py<PyComponent>> {
        let (module, component) = {
            let component = component.borrow();
            (component.0.clone(), component.1)
        };

        let module = {
            let module = module.bind(py);
            module.borrow()
        };

        let component = &module.inner[component];
        let name = name
            .cloned()
            .unwrap_or_else(|| PyString::new_bound(py, &component.name));
        let class = component.class.map(PyComponentClass::from);
        Self::add_component_impl_(slf, py, &name, class)
    }

    pub fn add_component(
        slf: &Bound<'_, PyModule_>,
        py: Python<'_>,
        name_or_component: Option<&Bound<'_, PyAny>>,
        component: Option<&Bound<'_, PyComponent>>,
        class: Option<PyComponentClass>,
    ) -> PyResult<Py<PyComponent>> {
        if let Some(component) = component {
            let name = name_or_component
                .map(|first_arg| {
                    first_arg.downcast::<PyString>().map_err(|_| {
                        let error_ty = first_arg.get_type();
                        PyTypeError::new_err(format!(
                            r#"expected name to be "str" not "{error_ty}""#
                        ))
                    })
                })
                .transpose()?;

            return Self::add_component_copy_(slf, py, component, name, class);
        }

        if let Some(first_arg) = name_or_component {
            if let Ok(component) = first_arg.downcast::<PyComponent>() {
                Self::add_component_copy_(slf, py, component, None, class)
            } else if let Ok(name) = first_arg.downcast::<PyString>() {
                Self::add_component_impl_(slf, py, name, class)
            } else {
                let error_ty = first_arg.get_type();
                Err(PyTypeError::new_err(
                    r#"expected string or component, not "{error_ty}""#,
                ))
            }
        } else {
            Err(PyValueError::new_err("component must have a name"))
        }
    }

    pub fn add_components(&mut self, components: &Bound<'_, PyMapping>) -> PyResult<()> {
        // iter_mapping_items!(for (name: PyString, component: PyComponent) in components => {
        //     self.add_component(name, component)?;
        // });

        Ok(())
    }
}

#[pyfunction]
pub fn json_loads(input: Bound<'_, PyString>) -> PyResult<Py<PyModule_>> {
    let py = input.py();

    let input = input.downcast::<PyString>()?;
    // let module: Module = map_serde_py_err!(serde_json::from_str(input.to_str()?))?;
    // let converter = ModuleConverter(py, module);

    // converter.convert()
    todo!()
}

#[pyfunction]
pub fn json_dumps(
    py: Python<'_>,
    module: &Bound<'_, PyModule_>,
    pretty: bool,
) -> PyResult<Py<PyString>> {
    // let converter = PyModuleConverter(module.clone());
    // let module = converter.convert().map_err(|err| match err {
    //     PyModuleConvertError::Python(err) => err,
    //     _ => PyValueError::new_err(format!("{err}")),
    // })?;

    // let json = if pretty {
    //     map_serde_py_err!(serde_json::to_string_pretty(&module))?
    // } else {
    //     map_serde_py_err!(serde_json::to_string(&module))?
    // };

    // let json = PyString::new_bound(py, json.as_str());
    // Ok(json.into_py(py))
    todo!()
}
