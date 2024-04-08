#![allow(unused)] // TODO: remove this!

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_core::arch::{component::ComponentBuilder, ComponentClass, Module};

use super::{PyComponent, PyComponentClass};

#[pyclass]
#[pyo3(name = "PyModule")]
pub struct PyModule_(pub(crate) Module);

enum NameOrComponent<'py> {
    Name(Bound<'py, PyString>),
    Component(Bound<'py, PyComponent>),
}

impl<'py> NameOrComponent<'py> {
    fn get_name(&self) -> PyResult<&Bound<'py, PyString>> {
        match self {
            NameOrComponent::Name(name) => Ok(name),
            NameOrComponent::Component(component) => {
                let error_ty = component.get_type();
                Err(PyTypeError::new_err(format!(
                    r#"expected name to be "str", not "{error_ty}""#
                )))
            }
        }
    }
}

impl<'py> FromPyObject<'py> for NameOrComponent<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(name) = ob.downcast::<PyString>() {
            Ok(NameOrComponent::Name(name.clone()))
        } else if let Ok(component) = ob.downcast::<PyComponent>() {
            Ok(NameOrComponent::Component(component.clone()))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                r#"expected string or component, not "{error_ty}""#,
            )))
        }
    }
}

enum ComponentClassOrStr<'py> {
    Class(Bound<'py, PyComponentClass>),
    Str(Bound<'py, PyString>),
}

impl<'py> ComponentClassOrStr<'py> {
    fn class(py: Python<'_>, class: PyComponentClass) -> PyResult<ComponentClassOrStr> {
        let class = Bound::new(py, class)?;
        Ok(ComponentClassOrStr::Class(class))
    }

    fn get_class(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponentClass>> {
        match self {
            ComponentClassOrStr::Class(class) => Ok(class.clone()),
            ComponentClassOrStr::Str(string) => {
                let class = string.to_str()?.to_lowercase();
                Bound::new(
                    py,
                    match class.as_str() {
                        "lut" => PyComponentClass::LUT,
                        "latch" | "ff" => PyComponentClass::LATCH,
                        _ => {
                            return Err(PyValueError::new_err(format!(
                                r#"unknown component class "{class}""#
                            )));
                        }
                    },
                )
            }
        }
    }
}

impl<'py> FromPyObject<'py> for ComponentClassOrStr<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(class) = ob.downcast::<PyComponentClass>() {
            Ok(ComponentClassOrStr::Class(class.clone()))
        } else if let Ok(string) = ob.downcast::<PyString>() {
            Ok(ComponentClassOrStr::Str(string.clone()))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                r#"expected component class or string, not "{error_ty}""#,
            )))
        }
    }
}

impl PyModule_ {
    fn _add_component_impl(
        slf: &Bound<'_, PyModule_>,
        py: Python<'_>,
        name: &Bound<'_, PyString>,
        class: Option<ComponentClassOrStr<'_>>,
    ) -> PyResult<PyComponent> {
        let mut module = slf.borrow_mut();

        // TODO: check for duplicate component names

        let mut builder = ComponentBuilder::new(&mut module.0).set_name(name.to_str()?);

        if let Some(class) = class {
            let class = class.get_class(py)?.borrow();
            builder.set_class(ComponentClass::from(*class));
        }

        let component = {
            let component = builder.finish();
            PyComponent::new(slf, component.key())
        };

        Ok(component)
    }

    fn _add_component_copy<'py>(
        slf: &Bound<'py, PyModule_>,
        py: Python<'py>,
        component: &Bound<'py, PyComponent>,
        name: Option<&Bound<'py, PyString>>,
        mut class: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<PyComponent> {
        let (module, component) = {
            let component = component.borrow();
            (component.module(py).clone().unbind(), component.key())
        };

        let module = {
            let module = module.bind(py);
            module.borrow()
        };

        let component = &module
            .0
            .get_component(component)
            .expect("component should be in module");
        let name = name
            .cloned()
            .unwrap_or_else(|| PyString::new_bound(py, &component.name()));

        if class.is_none() {
            class = component
                .class()
                .map(|class| ComponentClassOrStr::class(py, class.into()))
                .transpose()?
        }

        Self::_add_component_impl(slf, py, &name, class)
    }
}

#[pymethods]
impl PyModule_ {
    #[new]
    fn new(name: &Bound<'_, PyString>) -> PyResult<Self> {
        Ok(Self(Module::new(name.to_str()?)))
    }

    fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        PyString::new_bound(py, self.0.name())
    }

    fn copy(&self, name: Option<&Bound<'_, PyString>>) -> PyResult<Self> {
        let mut module = self.0.clone();

        if let Some(name) = name {
            module.rename(name.to_str()?);
        }

        Ok(Self(module))
    }

    #[pyo3(signature = (name=None, *, component=None, class_=None))]
    fn add_component(
        slf: &Bound<'_, PyModule_>,
        py: Python<'_>,
        name: Option<NameOrComponent<'_>>,
        component: Option<&Bound<'_, PyComponent>>,
        class_: Option<ComponentClassOrStr<'_>>,
    ) -> PyResult<PyComponent> {
        let class = class_;

        if let Some(component) = component {
            let name = name.as_ref().map(NameOrComponent::get_name).transpose()?;

            return Self::_add_component_copy(slf, py, component, name, class);
        }

        if let Some(first_arg) = name {
            match first_arg {
                NameOrComponent::Name(name) => Self::_add_component_impl(slf, py, &name, class),
                NameOrComponent::Component(component) => {
                    Self::_add_component_copy(slf, py, &component, None, class)
                }
            }
        } else {
            Err(PyValueError::new_err("component must have a name"))
        }
    }

    fn add_components(&mut self, components: &Bound<'_, PyMapping>) -> PyResult<()> {
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
