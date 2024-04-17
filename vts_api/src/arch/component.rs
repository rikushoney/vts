use std::str::FromStr;

use pyo3::{
    exceptions::{PyAttributeError, PyTypeError, PyValueError},
    prelude::*,
    types::{PyMapping, PyString},
};
use vts_core::arch::{
    component::ComponentKey,
    connection::{ComponentRefSelection, ConnectionBuilder},
    port::PortBuilder,
    prelude::*,
    reference::ComponentRefBuilder,
};

use super::{
    port::PyPortPins, reference::PyComponentRefSelection, IntoSignature, PyCheckerError,
    PyComponentRef, PyModule_, PyPort, PyPortClass, PyPortKind, SliceOrIndex,
};

wrap_enum!(
    PyComponentClass (name = "ComponentClass", help = "component class") => ComponentClass:
        LUT = Lut (alias = "lut"),
        LATCH = Latch (alias = "latch" | "ff"),
);

wrap_enum!(
    PyConnectionKind (name = "ConnectionKind", help = "connection kind") => ConnectionKind:
        DIRECT = Direct (alias = "direct" | "d"),
        COMPLETE = Complete (alias = "complete" | "c"),
        MUX = Mux (alias = "mux" | "m")
);

macro_rules! borrow_inner {
    ($slf:ident + $py:ident => $component:ident) => {
        let module = $slf.module($py).borrow();
        let inner = module.inner.borrow($py);
        let $component = inner
            .0
            .get_component($slf.key())
            .expect("component should be in module");
    };
}

#[pyclass(name = "Component")]
#[derive(Clone, Debug)]
pub struct PyComponent(Py<PyModule_>, ComponentKey);

impl PyComponent {
    pub(crate) fn new<'py>(
        module: Borrowed<'_, 'py, PyModule_>,
        component: ComponentKey,
    ) -> PyResult<Bound<'py, Self>> {
        let py = module.py();

        if let Some(component) = module.borrow().components.get(&component) {
            return Ok(component.bind(py).clone());
        }

        let py_component = Py::new(py, Self(module.as_unbound().clone_ref(py), component))?;

        module
            .borrow_mut()
            .components
            .insert(component, py_component.clone());

        Ok(py_component.bind(py).clone())
    }

    pub(crate) fn key(&self) -> ComponentKey {
        self.1
    }

    fn add_port_impl<'py>(
        &self,
        name: Borrowed<'_, 'py, PyString>,
        kind: PortKindOrStr<'py>,
        n_pins: Option<u32>,
        class: Option<PortClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyPort>> {
        let py = name.py();

        let port = {
            let parent = {
                borrow_inner!(self + py => component);
                component.key()
            };

            let module = self.module(py).borrow_mut();
            let mut inner = module.inner.borrow_mut(py);
            let mut checker = module.checker.borrow_mut(py);
            let kind = kind.get_kind(py)?.borrow();

            let mut builder = PortBuilder::new(&mut inner.0, &mut checker.0, parent)
                .set_name(name.to_str()?)
                .set_kind(PortKind::from(*kind));

            if let Some(n_pins) = n_pins {
                builder.set_n_pins(n_pins);
            }

            if let Some(class) = class {
                let class = class.get_class(py)?.borrow();
                builder.set_class(PortClass::from(*class));
            }

            builder.finish().map_err(PyCheckerError::from)?.key()
        };

        PyPort::new(self.module(py).as_borrowed(), port)
    }

    fn add_port_copy<'py>(
        &self,
        port: Borrowed<'_, 'py, PyPort>,
        name: Option<Borrowed<'_, 'py, PyString>>,
        kind: Option<PortKindOrStr<'py>>,
        n_pins: Option<u32>,
        mut class: Option<PortClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyPort>> {
        let py = port.py();

        let (module, port) = {
            let port = port.borrow();
            (port.module(py).clone().unbind(), port.key())
        };

        let module = module.bind(py).borrow();
        let inner = module.inner.borrow(py);

        let port = &inner.0.get_port(port).expect("port should be in module");

        let name = name
            .map(Borrowed::to_owned)
            .unwrap_or_else(|| PyString::new_bound(py, port.name()));

        let kind = if let Some(kind) = kind {
            kind
        } else {
            PortKindOrStr::new_kind(py, port.kind().into())?
        };

        let n_pins = n_pins.or_else(|| Some(port.n_pins()));

        if class.is_none() {
            class = port
                .class()
                .map(|class| PortClassOrStr::class(py, class.into()))
                .transpose()?;
        }

        self.add_port_impl(name.as_borrowed(), kind, n_pins, class)
    }
}

#[derive(FromPyObject)]
enum NameOrPort<'py> {
    #[pyo3(annotation = "str")]
    Name(Bound<'py, PyString>),
    #[pyo3(annotation = "Port")]
    Port(Bound<'py, PyPort>),
}

impl<'py> NameOrPort<'py> {
    fn get_name(&self) -> PyResult<&Bound<'py, PyString>> {
        match self {
            NameOrPort::Name(name) => Ok(name),
            _ => Err(PyTypeError::new_err("port must have a name")),
        }
    }
}

#[derive(FromPyObject)]
enum PortKindOrStr<'py> {
    #[pyo3(annotation = "PortKind")]
    Kind(Bound<'py, PyPortKind>),
    #[pyo3(annotation = "str")]
    Str(Bound<'py, PyString>),
}

impl<'py> PortKindOrStr<'py> {
    fn new_kind(py: Python<'py>, kind: PyPortKind) -> PyResult<PortKindOrStr<'py>> {
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

#[derive(FromPyObject)]
enum PortClassOrStr<'py> {
    #[pyo3(annotation = "PortClass")]
    Class(Bound<'py, PyPortClass>),
    #[pyo3(annotation = "str")]
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

#[derive(Clone, Debug)]
pub enum ComponentOrRef {
    Component(ComponentKey),
    Reference(PyComponentRefSelection),
}

#[derive(Clone, Debug)]
#[pyclass(name = "Signature")]
pub struct PySignature(pub(super) PyPortPins, pub(super) ComponentOrRef);

impl PySignature {
    pub fn get_reference_selection(&self) -> Option<&ComponentRefSelection> {
        match &self.1 {
            ComponentOrRef::Reference(selection) => Some(&selection.0),
            _ => None,
        }
    }
}

#[pymethods]
impl PyComponent {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        borrow_inner!(self + py => component);
        PyString::new_bound(py, component.name())
    }

    #[pyo3(name = "class_")]
    pub fn class(&self, py: Python<'_>) -> Option<PyComponentClass> {
        borrow_inner!(self + py => component);
        component.class().map(PyComponentClass::from)
    }

    #[pyo3(signature = (name=None, *, port=None, kind=None, n_pins=None, class_=None))]
    fn add_port<'py>(
        &self,
        name: Option<NameOrPort<'py>>,
        port: Option<&Bound<'py, PyPort>>,
        kind: Option<PortKindOrStr<'py>>,
        n_pins: Option<u32>,
        class_: Option<PortClassOrStr<'py>>,
    ) -> PyResult<Bound<'py, PyPort>> {
        let class = class_;

        if let Some(port) = port {
            let name = name.as_ref().map(NameOrPort::get_name).transpose()?;

            return self.add_port_copy(
                port.as_borrowed(),
                name.map(Bound::as_borrowed),
                kind,
                n_pins,
                class,
            );
        }

        if let Some(first_arg) = name {
            match first_arg {
                NameOrPort::Name(name) => {
                    let kind = kind.ok_or(PyValueError::new_err("port must have a kind"))?;
                    self.add_port_impl(name.as_borrowed(), kind, n_pins, class)
                }
                NameOrPort::Port(port) => {
                    self.add_port_copy(port.as_borrowed(), None, kind, n_pins, class)
                }
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
        component: &Bound<'py, PyComponent>,
        alias: Option<&Bound<'py, PyString>>,
        n_instances: Option<u32>,
    ) -> PyResult<Bound<'py, PyComponentRef>> {
        let py = component.py();

        let reference = {
            let module = self.module(py).borrow_mut();
            let mut inner = module.inner.borrow_mut(py);
            let mut checker = module.checker.borrow_mut(py);
            let component = component.borrow();

            let mut builder = ComponentRefBuilder::new(&mut inner.0, &mut checker.0, self.1)
                .set_component(component.1);

            if let Some(alias) = alias {
                builder.set_alias(alias.to_str()?);
            }

            if let Some(n_instances) = n_instances {
                builder.set_n_instances(n_instances);
            }

            builder.finish().map_err(PyCheckerError::from)?.key()
        };

        PyComponentRef::new(self.module(py).as_borrowed(), reference)
    }

    #[pyo3(signature = (source, sink, *, kind=None))]
    pub fn add_connection(
        &mut self,
        source: &Bound<'_, PySignature>,
        sink: &Bound<'_, PySignature>,
        kind: Option<PyConnectionKind>,
    ) -> PyResult<()> {
        let py = source.py();

        let module = self.module(py).borrow_mut();
        let mut inner = module.inner.borrow_mut(py);
        let mut checker = module.checker.borrow_mut(py);
        let source = source.borrow();
        let source_pins = &source.0;
        let source_selection = source.get_reference_selection().cloned();
        let sink = sink.borrow();
        let sink_pins = &sink.0;
        let sink_selection = sink.get_reference_selection().cloned();

        let mut builder = ConnectionBuilder::new(&mut inner.0, &mut checker.0, self.key())
            .set_source(source_pins.0.clone(), source_selection)
            .set_sink(sink_pins.0.clone(), sink_selection);

        if let Some(kind) = kind {
            builder.set_kind(ConnectionKind::from(kind));
        }

        builder.finish();
        Ok(())
    }

    // TODO: support getting references
    fn __getattr__<'py>(&self, port: &Bound<'py, PyString>) -> PyResult<Bound<'py, PyPort>> {
        let py = port.py();

        let port = {
            borrow_inner!(self + py => component);
            let port = port.to_str()?;
            component
                .find_port(port)
                .ok_or_else(|| {
                    let component = component.name();
                    PyAttributeError::new_err(format!(
                        r#"undefined port "{port}" referenced in "{component}""#,
                    ))
                })?
                .key()
        };

        PyPort::new(self.module(py).as_borrowed(), port)
    }

    fn __setattr__(
        slf: &Bound<'_, Self>,
        sink: &Bound<'_, PyString>,
        source: IntoSignature<'_>,
    ) -> PyResult<()> {
        let py = slf.py();

        let sink = {
            let port = slf.borrow().__getattr__(sink)?.borrow();
            let sink = port.__getitem__(py, SliceOrIndex::full(py))?;
            Bound::new(py, sink)?
        };

        slf.borrow_mut().add_connection(
            &source.into_signature()?,
            &sink,
            Some(PyConnectionKind::DIRECT),
        )
    }
}
