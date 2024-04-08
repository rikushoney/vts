#![allow(unused)] // TODO: remove this!

use std::ops::Deref;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyMapping, PyString};
use vts_core::arch::{
    component::{ComponentKey, ComponentRefKey},
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

impl PyComponent {
    pub(crate) fn new(module: &Bound<'_, PyModule_>, component: ComponentKey) -> Self {
        let module = module.clone().unbind();
        PyComponent(module, component)
    }

    pub(crate) fn key(&self) -> ComponentKey {
        self.1
    }

    fn _add_port_impl(
        &self,
        py: Python<'_>,
        name: &Bound<'_, PyString>,
        kind: PortKindOrStr<'_>,
        n_pins: Option<usize>,
        class: Option<PortClassOrStr<'_>>,
    ) -> PyResult<PyPort> {
        let mut module = self.module(py).borrow_mut();
        let parent = {
            let component = module
                .0
                .get_component(self.key())
                .expect("component should be in module");
            component.key()
        };

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

    fn _add_port_copy<'py>(
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
            .unwrap_or_else(|| PyString::new_bound(py, &port.name()));
        let kind = if let Some(kind) = kind {
            kind
        } else {
            PortKindOrStr::kind(py, port.kind().into())?
        };
        let n_pins = n_pins.or_else(|| Some(port.n_pins()));
        if class.is_none() {
            class = port
                .class()
                .and_then(|class| Some(PortClassOrStr::class(py, class.into())))
                .transpose()?;
        }

        self._add_port_impl(py, &name, kind, n_pins, class)
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
        let module = self.module(py).borrow();
        let component = module
            .0
            .get_component(self.key())
            .expect("component should be in module");

        PyString::new_bound(py, component.name())
    }

    #[pyo3(name = "class_")]
    pub fn class(&self, py: Python<'_>) -> Option<PyComponentClass> {
        let module = self.module(py).borrow();
        let component = module
            .0
            .get_component(self.key())
            .expect("component should be in module");

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

            return self._add_port_copy(py, port, name, kind, n_pins, class);
        }

        if let Some(first_arg) = name {
            match first_arg {
                NameOrPort::Name(name) => {
                    let kind = kind.ok_or(PyValueError::new_err("port must have a kind"))?;
                    self._add_port_impl(py, &name, kind, n_pins, class)
                }
                NameOrPort::Port(port) => self._add_port_copy(py, &port, None, kind, n_pins, class),
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

    pub fn add_reference(
        &mut self,
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<'_, PyString>>,
        n_instances: Option<usize>,
    ) -> PyResult<Py<PyComponentRef>> {
        // let py = component.py();

        // let alias = match alias {
        //     Some(alias) => alias.clone(),
        //     None => {
        //         let component = component.borrow();
        //         let alias = component.name.bind(py);
        //         PyString::new_bound(py, alias.to_str()?)
        //     }
        // };

        // let references = self.references.bind(py);
        // if references.deref().contains(alias.clone())? {
        //     return Err(PyValueError::new_err(format!(
        //         r#"component or alias "{alias}" already referenced in "{component}""#,
        //         alias = alias.to_str()?,
        //         component = self.name.bind(py).to_str()?
        //     )));
        // }

        // let reference = PyComponentRef::new(component, Some(&alias), n_instances);
        // let reference = Bound::new(py, reference)?;

        // references.deref().set_item(alias, reference.clone())?;

        // Ok(reference.unbind())
        todo!()
    }

    pub fn add_connection(
        &mut self,
        source_pins: &Bound<'_, PyPortPins>,
        sink_pins: &Bound<'_, PyPortPins>,
        source_component: Option<&Bound<'_, PyComponentRef>>,
        sink_component: Option<&Bound<'_, PyComponentRef>>,
    ) -> PyResult<Py<PyConnection>> {
        // let py = source_pins.py();

        // // TODO: check for duplicate connections?

        // let connections = self.connections.bind(py);
        // let connection = PyConnection::new(
        //     source_pins.clone(),
        //     sink_pins.clone(),
        //     source_component.cloned(),
        //     sink_component.cloned(),
        // );
        // let connection = Bound::new(py, connection)?;

        // connections.append(connection.clone())?;

        // Ok(connection.unbind())
        todo!()
    }
}

#[pyclass]
pub struct PyComponentRef(Py<PyModule_>, ComponentRefKey);

#[pymethods]
impl PyComponentRef {}

#[pyclass]
pub struct PyConnection {}

#[pymethods]
impl PyConnection {}
