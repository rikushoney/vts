use std::collections::HashMap;
use std::str::FromStr;

use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyMapping, PyString},
};
use vts_core::arch::{
    checker::Checker,
    component::ComponentBuilder,
    json,
    module::{ComponentId, ComponentRefId, PortId},
    toml, yaml, ComponentClass, Module,
};

use super::{prelude::*, PyCheckerError};

#[pyclass]
pub(crate) struct PyModuleInner(pub(crate) Module);

#[pyclass]
#[derive(Default)]
pub(crate) struct PyChecker(pub(crate) Checker);

#[pyclass(name = "Module")]
pub struct PyModule_ {
    pub(crate) inner: Py<PyModuleInner>,
    pub(crate) components: HashMap<ComponentId, Py<PyComponent>>,
    pub(crate) ports: HashMap<PortId, Py<PyPort>>,
    pub(crate) references: HashMap<ComponentRefId, Py<PyComponentRef>>,
    pub(crate) checker: Py<PyChecker>,
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
    pub(crate) fn new_wrap(py: Python<'_>, module: Module) -> PyResult<Self> {
        Py::new(py, PyChecker::default()).and_then(|checker| {
            Ok(Self {
                inner: Py::new(py, PyModuleInner(module))?,
                components: HashMap::default(),
                ports: HashMap::default(),
                references: HashMap::default(),
                checker,
            })
        })
    }

    fn with_inner<F, T>(slf: Borrowed<'_, '_, PyModule_>, mut exec: F) -> T
    where
        F: FnMut(&Module) -> T,
    {
        let py = slf.py();
        exec(&slf.borrow().inner.borrow(py).0)
    }

    fn add_component_impl<'py>(
        slf: Borrowed<'_, 'py, Self>,
        name: Borrowed<'_, 'py, PyString>,
        class: Option<ComponentClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyComponent>> {
        let py = slf.py();

        PyComponent::new(slf, {
            let module = slf.borrow_mut();
            let mut inner = module.inner.borrow_mut(py);
            let mut checker = module.checker.borrow_mut(py);

            let mut builder =
                ComponentBuilder::new(&mut inner.0, &mut checker.0).set_name(name.to_str()?);

            if let Some(class) = class {
                let class = class.get_class(py)?.borrow();
                builder.set_class(ComponentClass::from(*class));
            }

            builder.finish().map_err(PyCheckerError::from)?.unbind()
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
                (component.module(py).clone().unbind(), component.id())
            };

            let module = {
                let module = module.bind(py);
                module.borrow()
            };

            let inner = module.inner.borrow(py);

            let component = &inner
                .0
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
        let py = name.py();
        let module = Module::new(name.to_str()?);
        Self::new_wrap(py, module)
    }

    pub fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        PyString::new_bound(py, self.inner.borrow(py).0.name())
    }

    fn copy(&self, name: Option<&Bound<'_, PyString>>) -> PyResult<Self> {
        let module = self.inner.clone();

        if let Some(name) = name {
            let py = name.py();
            module.borrow_mut(py).0.rename(name.to_str()?);
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

    Py::new(py, PyModule_::new_wrap(py, module)?)
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

    Py::new(py, PyModule_::new_wrap(py, module)?)
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

    Py::new(py, PyModule_::new_wrap(py, module)?)
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
