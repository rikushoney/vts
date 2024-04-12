use std::collections::HashMap;
use std::str::FromStr;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyMapping, PyString};
use vts_core::arch::{
    component::{ComponentBuilder, ComponentKey},
    json,
    port::PortKey,
    reference::ComponentRefKey,
    toml, yaml, ComponentClass, Module,
};

use super::{PyComponent, PyComponentClass, PyComponentRef, PyPort};

#[pyclass]
#[pyo3(name = "PyModule")]
pub struct PyModule_ {
    pub(crate) inner: Module,
    pub(crate) components: HashMap<ComponentKey, Py<PyComponent>>,
    pub(crate) ports: HashMap<PortKey, Py<PyPort>>,
    pub(crate) references: HashMap<ComponentRefKey, Py<PyComponentRef>>,
}

#[derive(FromPyObject)]
enum NameOrComponent<'py> {
    #[pyo3(annotation = "str")]
    Name(Bound<'py, PyString>),
    #[pyo3(annotation = "Component")]
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

#[derive(FromPyObject)]
enum ComponentClassOrStr<'py> {
    #[pyo3(annotation = "ComponentClass")]
    Class(Bound<'py, PyComponentClass>),
    #[pyo3(annotation = "str")]
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

impl PyModule_ {
    pub(crate) fn new_wrap(module: Module) -> Self {
        Self {
            inner: module,
            components: HashMap::default(),
            ports: HashMap::default(),
            references: HashMap::default(),
        }
    }

    fn with_inner<F, T>(slf: Borrowed<'_, '_, PyModule_>, mut exec: F) -> T
    where
        F: FnMut(&Module) -> T,
    {
        exec(&slf.borrow().inner)
    }

    fn add_component_impl<'py>(
        slf: Borrowed<'_, 'py, Self>,
        name: Borrowed<'_, 'py, PyString>,
        class: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyComponent>> {
        let py = slf.py();

        PyComponent::new(slf, {
            let mut module = slf.borrow_mut();
            let mut builder = ComponentBuilder::new(&mut module.inner).set_name(name.to_str()?);

            if let Some(class) = class {
                let class = class.get_class(py)?.borrow();
                builder.set_class(ComponentClass::from(*class));
            }

            builder.finish().key()
        })
    }

    fn add_component_copy<'py>(
        slf: Borrowed<'_, 'py, Self>,
        component: Borrowed<'_, 'py, PyComponent>,
        name: Option<Borrowed<'_, 'py, PyString>>,
        mut class: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyComponent>> {
        let py = slf.py();

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
                .map(Borrowed::to_owned)
                .unwrap_or_else(|| PyString::new_bound(py, component.name()));

            if class.is_none() {
                class = component
                    .class()
                    .map(|class| ComponentClassOrStr::class(py, class.into()))
                    .transpose()?
            }

            (name, class)
        };

        Self::add_component_impl(slf, name.as_borrowed(), class)
    }
}

#[pymethods]
impl PyModule_ {
    #[new]
    pub fn new(name: &Bound<'_, PyString>) -> PyResult<Self> {
        let module = Module::new(name.to_str()?);
        Ok(Self::new_wrap(module))
    }

    pub fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
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
        slf: &Bound<'py, Self>,
        name: Option<NameOrComponent<'py>>,
        component: Option<&Bound<'py, PyComponent>>,
        class_: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyComponent>> {
        let slf = Borrowed::from(slf);
        let class = class_;

        if let Some(component) = component {
            let name = name.as_ref().map(NameOrComponent::get_name).transpose()?;

            return Self::add_component_copy(
                slf,
                component.as_borrowed(),
                name.map(Bound::as_borrowed),
                class,
            );
        }

        if let Some(first_arg) = name {
            match first_arg {
                NameOrComponent::Name(name) => {
                    Self::add_component_impl(slf, name.as_borrowed(), class)
                }
                NameOrComponent::Component(component) => {
                    Self::add_component_copy(slf, component.as_borrowed(), None, class)
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
pub fn json_loads(input: &Bound<'_, PyString>) -> PyResult<Py<PyModule_>> {
    let py = input.py();
    let input = input.downcast::<PyString>()?;

    let module: Module = json::from_str(input.to_str()?).map_err(|err| {
        PyValueError::new_err(format!(r#"failed parsing json with reason "{err}""#))
    })?;

    Py::new(py, PyModule_::new_wrap(module))
}

#[pyfunction]
pub fn json_dumps(module: &Bound<'_, PyModule_>, pretty: bool) -> PyResult<Py<PyString>> {
    let py = module.py();

    let json = PyModule_::with_inner(module.as_borrowed(), |module| {
        if pretty {
            json::to_string_pretty(module)
        } else {
            json::to_string(module)
        }
    })
    .map_err(|err| PyValueError::new_err(format!(r#"failed dumping json with reason "{err}""#)))?;

    let json = PyString::new_bound(py, json.as_str());
    Ok(json.into_py(py))
}

#[pyfunction]
pub fn yaml_loads(input: &Bound<'_, PyString>) -> PyResult<Py<PyModule_>> {
    let py = input.py();
    let input = input.downcast::<PyString>()?;

    let module: Module = yaml::from_str(input.to_str()?).map_err(|err| {
        PyValueError::new_err(format!(r#"failed parsing yaml with reason "{err}""#))
    })?;

    Py::new(py, PyModule_::new_wrap(module))
}

#[pyfunction]
pub fn yaml_dumps(module: &Bound<'_, PyModule_>) -> PyResult<Py<PyString>> {
    let py = module.py();

    let yaml = PyModule_::with_inner(module.as_borrowed(), yaml::to_string).map_err(|err| {
        PyValueError::new_err(format!(r#"failed dumping yaml with reason "{err}""#))
    })?;

    let yaml = PyString::new_bound(py, yaml.as_str());
    Ok(yaml.into_py(py))
}

#[pyfunction]
pub fn toml_loads(input: &Bound<'_, PyString>) -> PyResult<Py<PyModule_>> {
    let py = input.py();
    let input = input.downcast::<PyString>()?;

    let module: Module = toml::from_str(input.to_str()?).map_err(|err| {
        PyValueError::new_err(format!(r#"failed parsing toml with reason "{err}""#))
    })?;

    Py::new(py, PyModule_::new_wrap(module))
}

#[pyfunction]
pub fn toml_dumps(module: &Bound<'_, PyModule_>, pretty: bool) -> PyResult<Py<PyString>> {
    let py = module.py();

    let toml = PyModule_::with_inner(module.as_borrowed(), |module| {
        if pretty {
            toml::to_string_pretty(module)
        } else {
            toml::to_string(module)
        }
    })
    .map_err(|err| PyValueError::new_err(format!(r#"failed dumping toml with reason "{err}""#)))?;

    let toml = PyString::new_bound(py, toml.as_str());
    Ok(toml.into_py(py))
}
