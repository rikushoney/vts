use std::collections::HashMap;
use std::str::FromStr;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyMapping, PyString};
use vts_core::arch::{
    component::{ComponentBuilder, ComponentKey, ComponentRefKey},
    port::PortKey,
    ComponentClass, Module,
};

use super::{PyComponent, PyComponentClass, PyComponentRef, PyPort};

#[pyclass]
#[pyo3(name = "PyModule")]
pub struct PyModule {
    pub(crate) inner: Module,
    pub(crate) components: HashMap<ComponentKey, Py<PyComponent>>,
    pub(crate) ports: HashMap<PortKey, Py<PyPort>>,
    pub(crate) references: HashMap<ComponentRefKey, Py<PyComponentRef>>,
}

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
            ComponentClassOrStr::Str(string) => Bound::new(
                py,
                PyComponentClass::from_str(string.to_str()?).map_err(|class| {
                    PyValueError::new_err(format!(r#"unknown component class "{class}""#))
                })?,
            ),
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

impl PyModule {
    fn add_component_impl<'py>(
        slf: &Bound<'py, PyModule>,
        py: Python<'py>,
        name: &Bound<'py, PyString>,
        class: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyComponent>> {
        PyComponent::new(py, slf, {
            let mut module = slf.borrow_mut();

            // TODO: check for duplicate component names

            let mut builder = ComponentBuilder::new(&mut module.inner).set_name(name.to_str()?);

            if let Some(class) = class {
                let class = class.get_class(py)?.borrow();
                builder.set_class(ComponentClass::from(*class));
            }

            builder.finish().key()
        })
    }

    fn add_component_copy<'py>(
        slf: &Bound<'py, PyModule>,
        py: Python<'py>,
        component: &Bound<'py, PyComponent>,
        name: Option<&Bound<'py, PyString>>,
        mut class: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyComponent>> {
        let (name, class) = {
            let (module, component) = {
                let component = component.borrow();
                (component.module(py).clone().unbind(), component.key())
            };

            let module = {
                let module = module.bind(py);
                module.borrow()
            };

            let component = &module
                .inner
                .get_component(component)
                .expect("component should be in module");
            let name = name
                .cloned()
                .unwrap_or_else(|| PyString::new_bound(py, component.name()));

            if class.is_none() {
                class = component
                    .class()
                    .map(|class| ComponentClassOrStr::class(py, class.into()))
                    .transpose()?
            }

            (name, class)
        };

        Self::add_component_impl(slf, py, &name, class)
    }
}

#[pymethods]
impl PyModule {
    #[new]
    fn new(name: &Bound<'_, PyString>) -> PyResult<Self> {
        Ok(Self {
            inner: Module::new(name.to_str()?),
            components: HashMap::default(),
            ports: HashMap::default(),
            references: HashMap::default(),
        })
    }

    fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        PyString::new_bound(py, self.inner.name())
    }

    fn copy(&self, name: Option<&Bound<'_, PyString>>) -> PyResult<Self> {
        let mut module = self.inner.clone();

        if let Some(name) = name {
            module.rename(name.to_str()?);
        }

        // TODO: rebuild module
        todo!()
    }

    #[pyo3(signature = (name=None, *, component=None, class_=None))]
    fn add_component<'py>(
        slf: &Bound<'py, PyModule>,
        py: Python<'py>,
        name: Option<NameOrComponent<'py>>,
        component: Option<&Bound<'py, PyComponent>>,
        class_: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyComponent>> {
        let class = class_;

        if let Some(component) = component {
            let name = name.as_ref().map(NameOrComponent::get_name).transpose()?;

            return Self::add_component_copy(slf, py, component, name, class);
        }

        if let Some(first_arg) = name {
            match first_arg {
                NameOrComponent::Name(name) => Self::add_component_impl(slf, py, &name, class),
                NameOrComponent::Component(component) => {
                    Self::add_component_copy(slf, py, &component, None, class)
                }
            }
        } else {
            Err(PyValueError::new_err("component must have a name"))
        }
    }

    fn add_components(&mut self, components: &Bound<'_, PyMapping>) -> PyResult<()> {
        let _ = components;
        // iter_mapping_items!(for (name: PyString, component: PyComponent) in components => {
        //     self.add_component(name, component)?;
        // });

        Ok(())
    }
}

#[pyfunction]
pub fn json_loads(input: Bound<'_, PyString>) -> PyResult<Py<PyModule>> {
    let _ = input;
    // let py = input.py();

    // let input = input.downcast::<PyString>()?;
    // let module: Module = map_serde_py_err!(serde_json::from_str(input.to_str()?))?;
    // let converter = ModuleConverter(py, module);

    // converter.convert()
    todo!()
}

#[pyfunction]
pub fn json_dumps(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    pretty: bool,
) -> PyResult<Py<PyString>> {
    let _ = py;
    let _ = module;
    let _ = pretty;
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
