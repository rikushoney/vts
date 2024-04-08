#![allow(unused)] // TODO: remove this!

use std::ops::Deref;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyMapping, PyString};
use vts_core::arch::{
    component::{ComponentKey, ComponentRef, ComponentRefBuilder, ComponentRefKey},
    port::{PortBuilder, PortClass, PortKind},
    Component, ComponentClass,
};

use super::{module::PyModule_, port::PyPortPins, PyPort, PyPortClass, PyPortKind};

wrap_enum!(PyComponentClass => ComponentClass:
    LUT = Lut,
    LATCH = Latch,
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyComponent(Py<PyModule_>, ComponentKey);

macro_rules! extract_component {
    ($slf:ident + $py:ident => $comp:ident) => {
        let module = $slf.module($py).borrow();
        let $comp = module
            .0
            .get_component($slf.key())
            .expect("component should be in module");
    };
}

impl PyComponent {
    pub(crate) fn new(module: &Bound<'_, PyModule_>, component: ComponentKey) -> Self {
        let module = module.clone().unbind();
        PyComponent(module, component)
    }

    pub(crate) fn key(&self) -> ComponentKey {
        self.1
    }

    fn add_port_impl(
        &self,
        py: Python<'_>,
        name: &Bound<'_, PyString>,
        kind: PortKindOrStr<'_>,
        n_pins: Option<usize>,
        class: Option<PortClassOrStr<'_>>,
    ) -> PyResult<PyPort> {
        extract_component!(self + py => component);
        let parent = component.key();

        let mut module = self.module(py).borrow_mut();

        // TODO: check for duplicate port names

        let kind = kind.get_kind(py)?.borrow();

        let mut builder = PortBuilder::new(&mut module.0, parent)
            .set_name(name.to_str()?)
            .set_kind(PortKind::from(*kind));

        if let Some(n_pins) = n_pins {
            builder.set_n_pins(n_pins);
        }

        if let Some(class) = class {
            let class = class.get_class(py)?.borrow();
            builder.set_class(PortClass::from(*class));
        }

        let port = {
            let port = builder.finish();
            PyPort::new(self.module(py), port.key())
        };

        Ok(port)
    }

    fn add_port_copy<'py>(
        &self,
        py: Python<'py>,
        port: &Bound<'py, PyPort>,
        name: Option<&Bound<'py, PyString>>,
        kind: Option<PortKindOrStr<'py>>,
        n_pins: Option<usize>,
        mut class: Option<PortClassOrStr<'py>>,
    ) -> PyResult<PyPort> {
        let (module, port) = {
            let port = port.borrow();
            (port.module(py).clone().unbind(), port.key())
        };

        let module = {
            let module = module.bind(py);
            module.borrow()
        };

        let port = &module.0.get_port(port).expect("port should be in module");
        let name = name
            .cloned()
            .unwrap_or_else(|| PyString::new_bound(py, port.name()));
        let kind = if let Some(kind) = kind {
            kind
        } else {
            PortKindOrStr::kind(py, port.kind().into())?
        };
        let n_pins = n_pins.or_else(|| Some(port.n_pins()));
        if class.is_none() {
            class = port
                .class()
                .map(|class| PortClassOrStr::class(py, class.into()))
                .transpose()?;
        }

        self.add_port_impl(py, &name, kind, n_pins, class)
    }
}

enum NameOrPort<'py> {
    Name(Bound<'py, PyString>),
    Port(Bound<'py, PyPort>),
}

impl<'py> NameOrPort<'py> {
    fn get_name(&self) -> PyResult<&Bound<'py, PyString>> {
        match self {
            NameOrPort::Name(name) => Ok(name),
            NameOrPort::Port(port) => {
                let error_ty = port.get_type();
                Err(PyTypeError::new_err(format!(
                    r#"expected name to be "str", not "{error_ty}""#
                )))
            }
        }
    }
}

impl<'py> FromPyObject<'py> for NameOrPort<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(name) = ob.downcast::<PyString>() {
            Ok(NameOrPort::Name(name.clone()))
        } else if let Ok(port) = ob.downcast::<PyPort>() {
            Ok(NameOrPort::Port(port.clone()))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                r#"expected name or port, not "{error_ty}""#
            )))
        }
    }
}

enum PortKindOrStr<'py> {
    Kind(Bound<'py, PyPortKind>),
    Str(Bound<'py, PyString>),
}

impl<'py> PortKindOrStr<'py> {
    fn kind(py: Python<'py>, kind: PyPortKind) -> PyResult<PortKindOrStr<'py>> {
        let kind = Bound::new(py, kind)?;
        Ok(PortKindOrStr::Kind(kind))
    }

    fn get_kind(&self, py: Python<'py>) -> PyResult<Bound<'py, PyPortKind>> {
        match self {
            PortKindOrStr::Kind(kind) => Ok(kind.clone()),
            PortKindOrStr::Str(string) => {
                let kind = string.to_str()?.to_lowercase();
                Bound::new(
                    py,
                    match kind.as_str() {
                        "i" | "in" | "input" => PyPortKind::INPUT,
                        "o" | "out" | "output" => PyPortKind::OUTPUT,
                        _ => {
                            return Err(PyValueError::new_err(format!(
                                r#"unknown port kind "{kind}""#
                            )));
                        }
                    },
                )
            }
        }
    }
}

impl<'py> FromPyObject<'py> for PortKindOrStr<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(kind) = ob.downcast::<PyPortKind>() {
            Ok(PortKindOrStr::Kind(kind.clone()))
        } else if let Ok(string) = ob.downcast::<PyString>() {
            Ok(PortKindOrStr::Str(string.clone()))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                "expected port kind or string, not {error_ty}"
            )))
        }
    }
}

enum PortClassOrStr<'py> {
    Class(Bound<'py, PyPortClass>),
    Str(Bound<'py, PyString>),
}

impl<'py> PortClassOrStr<'py> {
    fn class(py: Python<'py>, class: PyPortClass) -> PyResult<PortClassOrStr> {
        let class = Bound::new(py, class)?;
        Ok(PortClassOrStr::Class(class))
    }

    fn get_class(&self, py: Python<'py>) -> PyResult<Bound<'py, PyPortClass>> {
        match self {
            PortClassOrStr::Class(class) => Ok(class.clone()),
            PortClassOrStr::Str(string) => {
                let class = string.to_str()?.to_lowercase();
                Bound::new(
                    py,
                    match class.as_str() {
                        "lut_in" => PyPortClass::LUT_IN,
                        "lut_out" => PyPortClass::LUT_OUT,
                        "latch_in" | "ff_in" => PyPortClass::LATCH_IN,
                        "latch_out" | "ff_out" => PyPortClass::LATCH_OUT,
                        _ => {
                            return Err(PyValueError::new_err(format!(
                                r#"unknown port class "{class}""#
                            )));
                        }
                    },
                )
            }
        }
    }
}

impl<'py> FromPyObject<'py> for PortClassOrStr<'py> {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(class) = ob.downcast::<PyPortClass>() {
            Ok(PortClassOrStr::Class(class.clone()))
        } else if let Ok(string) = ob.downcast::<PyString>() {
            Ok(PortClassOrStr::Str(string.clone()))
        } else {
            let error_ty = ob.get_type();
            Err(PyTypeError::new_err(format!(
                "expected port class or string, not {error_ty}"
            )))
        }
    }
}

#[pymethods]
impl PyComponent {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        extract_component!(self + py => component);
        PyString::new_bound(py, component.name())
    }

    #[pyo3(name = "class_")]
    pub fn class(&self, py: Python<'_>) -> Option<PyComponentClass> {
        extract_component!(self + py => component);
        component.class().map(PyComponentClass::from)
    }

    #[pyo3(signature = (name=None, *, port=None, kind=None, n_pins=None, class_=None))]
    fn add_port(
        &self,
        py: Python<'_>,
        name: Option<NameOrPort<'_>>,
        port: Option<&Bound<'_, PyPort>>,
        kind: Option<PortKindOrStr<'_>>,
        n_pins: Option<usize>,
        class_: Option<PortClassOrStr<'_>>,
    ) -> PyResult<PyPort> {
        let class = class_;

        if let Some(port) = port {
            let name = name.as_ref().map(NameOrPort::get_name).transpose()?;

            return self.add_port_copy(py, port, name, kind, n_pins, class);
        }

        if let Some(first_arg) = name {
            match first_arg {
                NameOrPort::Name(name) => {
                    let kind = kind.ok_or(PyValueError::new_err("port must have a kind"))?;
                    self.add_port_impl(py, &name, kind, n_pins, class)
                }
                NameOrPort::Port(port) => self.add_port_copy(py, &port, None, kind, n_pins, class),
            }
        } else {
            Err(PyValueError::new_err("port must have a name"))
        }
    }

    pub fn add_ports(&mut self, ports: &Bound<'_, PyMapping>) -> PyResult<()> {
        // iter_mapping_items!(for (name: PyString, port: PyPort) in ports => {
        //     self.add_port(name, port)?;
        // });

        Ok(())
    }

    #[pyo3(signature = (component, *, alias=None, n_instances=None))]
    pub fn add_reference(
        &mut self,
        py: Python<'_>,
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<'_, PyString>>,
        n_instances: Option<usize>,
    ) -> PyResult<PyComponentRef> {
        let mut module = self.module(py).borrow_mut();
        let component = component.borrow();

        let mut builder = ComponentRefBuilder::new(&mut module.0, component.1);

        if let Some(alias) = alias {
            builder.set_alias(alias.to_str()?);
        }

        if let Some(n_instances) = n_instances {
            builder.set_n_instances(n_instances);
        }

        let reference = {
            let reference = builder.finish();
            PyComponentRef::new(self.module(py), reference.key())
        };

        Ok(reference)
    }

    pub fn add_connection(
        &mut self,
        source_pins: &Bound<'_, PyPortPins>,
        sink_pins: &Bound<'_, PyPortPins>,
        source_component: Option<&Bound<'_, PyComponentRef>>,
        sink_component: Option<&Bound<'_, PyComponentRef>>,
    ) -> PyResult<Py<PyConnection>> {
        todo!()
    }
}

#[pyclass]
pub struct PyComponentRef(Py<PyModule_>, ComponentRefKey);

impl PyComponentRef {
    pub(crate) fn new(module: &Bound<'_, PyModule_>, reference: ComponentRefKey) -> Self {
        let module = module.clone().unbind();
        Self(module, reference)
    }

    pub(crate) fn key(&self) -> ComponentRefKey {
        self.1
    }
}

macro_rules! extract_reference {
    ($slf:ident + $py:ident => $ref:ident) => {
        let module = $slf.module($py).borrow();
        let $ref = module
            .0
            .get_reference($slf.key())
            .expect("reference should be in module");
    };
}

#[pymethods]
impl PyComponentRef {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn component<'py>(&self, py: Python<'py>) -> PyComponent {
        extract_reference!(self + py => reference);
        PyComponent::new(self.module(py), reference.component().key())
    }

    pub fn alias<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyString>> {
        extract_reference!(self + py => reference);

        if let Some(alias) = reference.alias() {
            Some(PyString::new_bound(py, alias))
        } else {
            None
        }
    }

    pub fn alias_or_name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        extract_reference!(self + py => reference);
        PyString::new_bound(py, reference.alias_or_name())
    }

    pub fn n_instances(&self, py: Python<'_>) -> usize {
        extract_reference!(self + py => reference);
        reference.n_instances()
    }
}

#[pyclass]
pub struct PyConnection {}

#[pymethods]
impl PyConnection {}
