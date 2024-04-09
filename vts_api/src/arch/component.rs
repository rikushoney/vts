use std::str::FromStr;

use pyo3::exceptions::{PyAttributeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyMapping, PyString};
use vts_core::arch::{
    component::{
        connection::{ConnectionBuilder, ConnectionKind},
        ComponentKey, ComponentRefBuilder,
    },
    port::{PortBuilder, PortClass, PortKind},
    ComponentClass,
};

use super::{
    port::{ComponentOrRef, SliceOrIndex},
    PyComponentRef, PyModule as PyModule_, PyPort, PyPortClass, PyPortKind, PyPortSelection,
};

wrap_enum!(
    PyComponentClass as "component class" => ComponentClass:
        LUT = Lut ("lut"),
        LATCH = Latch ("latch" | "ff"),
);

wrap_enum!(
    PyConnectionKind as "connection kind" => ConnectionKind:
        DIRECT = Direct ("direct"),
        COMPLETE = Complete ("complete"),
        MUX = Mux ("mux")
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyComponent(Py<PyModule_>, ComponentKey);

macro_rules! get_component {
    ($slf:ident + $py:ident => $comp:ident) => {
        let module = $slf.module($py).borrow();
        let $comp = module
            .inner
            .get_component($slf.key())
            .expect("component should be in module")
            .clone();
    };
}

impl PyComponent {
    pub(crate) fn new<'py>(
        py: Python<'py>,
        module: &Bound<'py, PyModule_>,
        component: ComponentKey,
    ) -> PyResult<Bound<'py, Self>> {
        if let Some(component) = module.borrow().components.get(&component) {
            Ok(component.bind(py).clone())
        } else {
            let py_component = Py::new(py, Self(module.clone().unbind(), component))?;

            module
                .borrow_mut()
                .components
                .insert(component, py_component.clone());

            Ok(py_component.bind(py).clone())
        }
    }

    pub(crate) fn key(&self) -> ComponentKey {
        self.1
    }

    fn add_port_impl<'py>(
        &self,
        py: Python<'py>,
        name: &Bound<'py, PyString>,
        kind: PortKindOrStr<'py>,
        n_pins: Option<usize>,
        class: Option<PortClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyPort>> {
        let parent = {
            get_component!(self + py => component);
            component.key()
        };

        let mut module = self.module(py).borrow_mut();

        // TODO: check for duplicate port names

        let kind = kind.get_kind(py)?.borrow();

        let mut builder = PortBuilder::new(&mut module.inner, parent)
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
            PyPort::new(py, self.module(py), port.key())?
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
    ) -> PyResult<Bound<'py, PyPort>> {
        let (module, port) = {
            let port = port.borrow();
            (port.module(py).clone().unbind(), port.key())
        };

        let module = {
            let module = module.bind(py);
            module.borrow()
        };

        let port = &module
            .inner
            .get_port(port)
            .expect("port should be in module");
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
            PortKindOrStr::Str(string) => Bound::new(
                py,
                PyPortKind::from_str(string.to_str()?).map_err(|kind| {
                    PyValueError::new_err(format!(r#"unknown port kind "{kind}""#))
                })?,
            ),
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
            PortClassOrStr::Str(string) => Bound::new(
                py,
                PyPortClass::from_str(string.to_str()?).map_err(|class| {
                    PyValueError::new_err(format!(r#"unknown port class "{class}""#))
                })?,
            ),
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
        get_component!(self + py => component);
        PyString::new_bound(py, component.name())
    }

    #[pyo3(name = "class_")]
    pub fn class(&self, py: Python<'_>) -> Option<PyComponentClass> {
        get_component!(self + py => component);
        component.class().map(PyComponentClass::from)
    }

    #[pyo3(signature = (name=None, *, port=None, kind=None, n_pins=None, class_=None))]
    fn add_port<'py>(
        &self,
        py: Python<'py>,
        name: Option<NameOrPort<'py>>,
        port: Option<&Bound<'py, PyPort>>,
        kind: Option<PortKindOrStr<'py>>,
        n_pins: Option<usize>,
        class_: Option<PortClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyPort>> {
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
        let _ = ports;
        // iter_mapping_items!(for (name: PyString, port: PyPort) in ports => {
        //     self.add_port(name, port)?;
        // });

        Ok(())
    }

    #[pyo3(signature = (component, *, alias=None, n_instances=None))]
    pub fn add_reference<'py>(
        &mut self,
        py: Python<'py>,
        component: &Bound<'py, PyComponent>,
        alias: Option<&Bound<'py, PyString>>,
        n_instances: Option<usize>,
    ) -> PyResult<Bound<'py, PyComponentRef>> {
        PyComponentRef::new(py, self.module(py), {
            let mut module = self.module(py).borrow_mut();
            let component = component.borrow();
            let mut builder = ComponentRefBuilder::new(&mut module.inner, component.1);

            if let Some(alias) = alias {
                builder.set_alias(alias.to_str()?);
            }

            if let Some(n_instances) = n_instances {
                builder.set_n_instances(n_instances);
            }

            builder.finish().key()
        })
    }

    #[pyo3(signature = (source, sink, *, kind=None))]
    pub fn add_connection(
        &mut self,
        py: Python<'_>,
        source: &Bound<'_, PyPortSelection>,
        sink: &Bound<'_, PyPortSelection>,
        kind: Option<PyConnectionKind>,
    ) -> PyResult<()> {
        let mut module = self.module(py).borrow_mut();

        let source = source.borrow();
        let source_pins = &source.1;

        let source_component = match source.0 {
            ComponentOrRef::Component(_) => None,
            ComponentOrRef::Ref(reference) => Some(reference),
        };

        let sink = sink.borrow();
        let sink_pins = &sink.1;

        let sink_component = match sink.0 {
            ComponentOrRef::Component(_) => None,
            ComponentOrRef::Ref(reference) => Some(reference),
        };

        let mut builder = ConnectionBuilder::new(&mut module.inner, self.key())
            .set_source(source_pins.0.clone(), source_component)
            .set_sink(sink_pins.0.clone(), sink_component);

        if let Some(kind) = kind {
            builder.set_kind(ConnectionKind::from(kind));
        }

        builder.finish();
        Ok(())
    }

    fn __getattr__<'py>(
        &self,
        py: Python<'py>,
        port: &Bound<'py, PyString>,
    ) -> PyResult<Bound<'py, PyPort>> {
        let port = port.to_str()?;
        get_component!(self + py => component);

        let port = component.find_port(port).ok_or_else(|| {
            let component = component.name();
            PyAttributeError::new_err(format!(
                r#"undefined port "{port}" referenced in "{component}""#,
            ))
        })?;

        PyPort::new(py, self.module(py), port.key())
    }

    fn __setattr__(
        &mut self,
        py: Python<'_>,
        source: &Bound<'_, PyString>,
        sink: &Bound<'_, PyPortSelection>,
    ) -> PyResult<()> {
        let port = self.__getattr__(py, source)?.borrow();

        let source = {
            let source = port.__getitem__(py, SliceOrIndex::full(py))?;
            Bound::new(py, source)?
        };

        self.add_connection(py, &source, sink, Some(PyConnectionKind::DIRECT))
    }
}
