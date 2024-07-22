use std::str::FromStr;

use pyo3::prelude::*;
use vts_core::arch::{builder::prelude::*, prelude::*};

use super::prelude::*;

wrap_enum!(
    PyComponentClass (name = "ComponentClass", help = "component class") => ComponentClass:
        LUT = Lut (alias = "lut"),
        LATCH = Latch (alias = "latch" | "ff"),
);

macro_rules! borrow_inner {
    ($slf:ident + $py:ident => $component:ident) => {
        let module = $slf.module($py).borrow();
        let inner = module.inner.borrow($py);
        let $component = inner
            .0
            .get_component($slf.id())
            .expect("component should be in module");
    };
}

#[pyclass(name = "Component")]
#[derive(Clone, Debug)]
pub struct PyComponent(Py<PyModule_>, ComponentId);

impl PyComponent {
    pub(crate) fn new<'py>(
        module: Borrowed<'_, 'py, PyModule_>,
        component: ComponentId,
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

    pub(crate) fn id(&self) -> ComponentId {
        self.1
    }

    fn find_port<'py>(
        &self,
        port: Borrowed<'_, 'py, PyString>,
    ) -> PyResult<Option<Bound<'py, PyPort>>> {
        // TODO: cache
        let py = port.py();

        borrow_inner!(self + py => component);
        let port = port.to_str()?;

        component
            .find_port(port)
            .map(|port| PyPort::new(self.module(py).as_borrowed(), port.unbind()))
            .transpose()
    }

    fn find_reference<'py>(
        &self,
        reference: Borrowed<'_, 'py, PyString>,
    ) -> PyResult<Option<Bound<'py, PyComponentRef>>> {
        // TODO: cache
        let py = reference.py();

        borrow_inner!(self + py => component);
        let reference = reference.to_str()?;

        component
            .find_reference(reference)
            .map(|reference| PyComponentRef::new(self.module(py).as_borrowed(), reference.id()))
            .transpose()
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
                component.unbind()
            };

            let module = self.module(py).borrow_mut();
            let mut inner = module.inner.borrow_mut(py);
            let mut checker = module.checker.borrow_mut(py);
            let kind = kind.get_kind(py)?.borrow();

            let mut builder = inner
                .0
                .add_port(&mut checker.0, parent)
                .set_name(name.to_str()?)
                .set_kind(PortKind::from(*kind));

            if let Some(n_pins) = n_pins {
                builder.set_n_pins(n_pins);
            }

            if let Some(class) = class {
                let class = class.get_class(py)?.borrow();
                builder.set_class(PortClass::from(*class));
            }

            builder.finish().map_err(PyCheckerError::from)?.unbind()
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
            (port.module(py).clone().unbind(), port.id())
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
        &self,
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

            builder.finish().map_err(PyCheckerError::from)?.unbind()
        };

        PyComponentRef::new(self.module(py).as_borrowed(), reference)
    }

    #[pyo3(signature = (source, sink, *, kind=None))]
    pub fn add_connection(
        &self,
        source: &Bound<'_, PySignature>,
        sink: &Bound<'_, PySignature>,
        kind: Option<PyConnectionKind>,
    ) -> PyResult<()> {
        let py = source.py();

        let source = source.borrow();
        let source_selection = source.get_reference(py);

        let sink = sink.borrow();
        let sink_selection = sink.get_reference(py);

        let module = self.module(py).borrow_mut();
        let mut inner = module.inner.borrow_mut(py);
        let mut checker = module.checker.borrow_mut(py);

        let mut builder = inner
            .0
            .add_connection(&mut checker.0, self.id())
            .set_source(source.pins.1.clone(), source_selection)
            .set_sink(sink.pins.1.clone(), sink_selection);

        if let Some(kind) = kind {
            builder.set_kind(ConnectionKind::from(kind));
        }

        builder.finish().map_err(PyLinkerError::from)?;
        Ok(())
    }

    fn __getattr__<'py>(
        &self,
        port_or_reference: &Bound<'py, PyString>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = port_or_reference.py();

        if let Some(port) = self.find_port(port_or_reference.as_borrowed())? {
            return Ok(port.as_any().clone());
        }

        if let Some(reference) = self.find_reference(port_or_reference.as_borrowed())? {
            return Ok(reference.as_any().clone());
        }

        let component = self.name(py);

        Err(PyAttributeError::new_err(format!(
            r#"undefined port or component "{port_or_reference}" referenced in "{component}""#
        )))
    }

    fn __setattr__(
        slf: &Bound<'_, Self>,
        sink: &Bound<'_, PyString>,
        source: Connector<'_>,
    ) -> PyResult<()> {
        let py = slf.py();

        let sink = {
            let component = slf.borrow();

            let port = component
                .find_port(sink.as_borrowed())?
                .ok_or_else(|| {
                    let component = slf.borrow().name(py);
                    PyAttributeError::new_err(format!(
                        r#"undefined port "{sink}" referenced in "{component}""#,
                    ))
                })?
                .borrow();

            let sink = port.__getitem__(py, SliceOrIndex::full(py))?;
            Bound::new(py, sink)?
        };

        source.connect(slf.as_borrowed(), IntoSignature::Signature(sink.unbind()))
    }
}
