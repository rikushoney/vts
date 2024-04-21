use std::ops::Range;

use pyo3::{
    exceptions::{PyAttributeError, PyTypeError},
    prelude::*,
    types::{PyString, PyTuple},
};
use vts_core::arch::{
    connection::{ComponentRefSelection, ConnectionKind},
    module::ComponentId,
    reference::ReferenceRange,
};

use super::{prelude::*, reference};

wrap_enum!(
    PyConnectionKind (name = "ConnectionKind", help = "connection kind") => ConnectionKind:
        DIRECT = Direct (alias = "direct" | "d"),
        COMPLETE = Complete (alias = "complete" | "c"),
        MUX = Mux (alias = "mux" | "m")
);

#[pyclass(name = "ComponentRefPort")]
#[derive(Clone, Debug)]
pub struct PyComponentRefPort {
    components: Py<PyComponentRefSelection>,
    port: Py<PyPort>,
}

impl PyComponentRefPort {
    pub fn new(
        components: Borrowed<'_, '_, PyComponentRefSelection>,
        port: Borrowed<'_, '_, PyPort>,
    ) -> Self {
        Self {
            components: components.to_owned().unbind(),
            port: port.to_owned().unbind(),
        }
    }

    pub fn components<'py>(&self, py: Python<'py>) -> &Bound<'py, PyComponentRefSelection> {
        self.components.bind(py)
    }

    pub fn port<'py>(&self, py: Python<'py>) -> &Bound<'py, PyPort> {
        self.port.bind(py)
    }
}

#[pymethods]
impl PyComponentRefPort {
    pub fn __getitem__(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PySignature> {
        let port = self.port.bind(py).borrow();
        let pins = port.select(py, index)?;

        Ok(PySignature {
            component_or_reference: ComponentOrRef::Reference(self.components.borrow(py).clone()),
            pins,
        })
    }

    pub fn __setitem__(
        &self,
        py: Python<'_>,
        sink: SliceOrIndex<'_>,
        source: Connector<'_>,
    ) -> PyResult<()> {
        let sink = Bound::new(py, self.__getitem__(py, sink)?)?;
        let selection = self.components.bind(py).borrow();
        let reference = selection.reference(py).borrow();

        source.connect(
            reference.parent(py)?.as_borrowed(),
            IntoSignature::Signature(sink.unbind()),
        )
    }
}

#[derive(Clone, Debug)]
pub enum ComponentOrRef {
    Component(ComponentId),
    Reference(PyComponentRefSelection),
}

impl ComponentOrRef {
    pub fn len(&self, py: Python<'_>) -> usize {
        match self {
            Self::Component(_) => 1,
            Self::Reference(selection) => {
                let Range { start, end } = selection
                    .range
                    .expand(selection.reference(py).borrow().n_instances(py));

                (end - start) as usize
            }
        }
    }
}

#[derive(Clone, Debug)]
#[pyclass(name = "Signature")]
pub struct PySignature {
    component_or_reference: ComponentOrRef,
    pub pins: PyPortPins,
}

impl PySignature {
    pub fn new_component(component: ComponentId, pins: PyPortPins) -> Self {
        Self {
            component_or_reference: ComponentOrRef::Component(component),
            pins,
        }
    }

    pub fn new_reference(reference: PyComponentRefSelection, pins: PyPortPins) -> Self {
        Self {
            component_or_reference: ComponentOrRef::Reference(reference),
            pins,
        }
    }

    pub fn get_reference(&self, py: Python<'_>) -> Option<ComponentRefSelection> {
        match &self.component_or_reference {
            ComponentOrRef::Reference(selection) => Some(ComponentRefSelection::new(
                selection.reference(py).borrow().id(),
                selection.range.clone(),
            )),
            _ => None,
        }
    }

    pub fn component_count(&self, py: Python<'_>) -> u32 {
        self.counts(py).0
    }

    pub fn pin_count(&self, py: Python<'_>) -> u32 {
        self.counts(py).1
    }

    pub fn counts(&self, py: Python<'_>) -> (u32, u32) {
        let component_count = self.component_count(py);
        (component_count, component_count * self.pins.len(py))
    }
}

#[derive(Clone, Debug)]
#[pyclass(name = "ComponentRefSelection")]
pub struct PyComponentRefSelection {
    reference: Py<PyComponentRef>,
    pub range: ReferenceRange,
}

impl PyComponentRefSelection {
    pub fn new(reference: Borrowed<'_, '_, PyComponentRef>, range: ReferenceRange) -> Self {
        Self {
            reference: reference.to_owned().unbind(),
            range,
        }
    }

    pub fn reference<'py>(&self, py: Python<'py>) -> &Bound<'py, PyComponentRef> {
        self.reference.bind(py)
    }
}

#[pymethods]
impl PyComponentRefSelection {
    pub fn __getattr__(&self, port: &Bound<'_, PyString>) -> PyResult<PyComponentRefPort> {
        let py = port.py();
        let reference = self.reference.bind(py);

        let port = {
            let reference = reference.borrow();
            reference::borrow_inner!(reference + py => reference);

            reference
                .component()
                .find_port(port.to_str()?)
                .ok_or(PyAttributeError::new_err(format!(
                    r#"undefined port "{port}" referenced in "{component}""#,
                    component = reference.component().name()
                )))?
                .unbind()
        };

        let port = PyPort::new(reference.borrow().module(py).as_borrowed(), port)?;
        let reference = PyComponentRef::__getitem__(reference, py, SliceOrIndex::full(py))?;

        Ok(PyComponentRefPort {
            components: Py::new(py, reference)?,
            port: port.unbind(),
        })
    }

    pub fn __setattr__(&self, sink: &Bound<'_, PyString>, source: Connector<'_>) -> PyResult<()> {
        let py = sink.py();

        let sink = {
            let sink = self.__getattr__(sink)?;
            Bound::new(py, sink.__getitem__(py, SliceOrIndex::full(py))?)?
        };

        let reference = self.reference.bind(py).borrow();
        let parent = reference.parent(py)?;

        source.connect(
            parent.as_borrowed(),
            IntoSignature::Signature(sink.unbind()),
        )
    }
}

#[derive(Clone, Debug, FromPyObject)]
pub enum IntoSignature {
    #[pyo3(annotation = "Signature")]
    Signature(Py<PySignature>),
    #[pyo3(annotation = "ComponentRefPort")]
    PortRef(Py<PyComponentRefPort>),
    #[pyo3(annotation = "Port")]
    Port(Py<PyPort>),
}

impl IntoSignature {
    pub fn into_signature(self, py: Python<'_>) -> PyResult<Py<PySignature>> {
        match self {
            Self::Signature(signature) => Ok(signature),
            Self::PortRef(reference) => Py::new(
                py,
                reference
                    .borrow(py)
                    .__getitem__(py, SliceOrIndex::full(py))?,
            ),
            Self::Port(port) => {
                Py::new(py, port.borrow(py).__getitem__(py, SliceOrIndex::full(py))?)
            }
        }
    }
}

#[pyclass(name = "Direct")]
pub struct PyDirect(IntoSignature);

#[pyfunction]
pub fn direct(py: Python<'_>, connector: Connector<'_>) -> PyResult<PyDirect> {
    connector.get_signature(py).map(PyDirect)
}

#[pyclass(name = "Complete")]
pub struct PyComplete(IntoSignature);

#[pyfunction]
pub fn complete(py: Python<'_>, connector: Connector<'_>) -> PyResult<PyComplete> {
    connector.get_signature(py).map(PyComplete)
}

#[pyclass(name = "Mux")]
pub struct PyMux(IntoSignature);

#[pyfunction]
pub fn mux(py: Python<'_>, connector: Connector<'_>) -> PyResult<PyMux> {
    connector.get_signature(py).map(PyMux)
}

#[pyclass(name = "Concat")]
pub struct PyConcat(Vec<IntoSignature>);

#[pyfunction]
#[pyo3(signature = (*connectors))]
pub fn concat(connectors: &Bound<'_, PyTuple>) -> PyResult<PyConcat> {
    connectors
        .iter()
        .try_fold(Vec::new(), |mut signatures, signature| {
            signature.extract::<IntoSignature>().map(|signature| {
                signatures.push(signature);
                signatures
            })
        })
        .map(PyConcat)
}

#[derive(Clone, Debug, FromPyObject)]
pub enum Connector<'py> {
    #[pyo3(transparent)]
    Signature(IntoSignature),
    #[pyo3(annotation = "Direct")]
    Direct(Bound<'py, PyDirect>),
    #[pyo3(annotation = "Complete")]
    Complete(Bound<'py, PyComplete>),
    #[pyo3(annotation = "Mux")]
    Mux(Bound<'py, PyMux>),
    #[pyo3(annotation = "Concat")]
    Concat(Bound<'py, PyConcat>),
}

impl<'py> Connector<'py> {
    pub fn to_object(&self, py: Python<'py>) -> PyResult<PyObject> {
        match self {
            Self::Signature(signature) => Ok(signature.clone().into_signature(py)?.to_object(py)),
            Self::Direct(direct) => Ok(direct.to_object(py)),
            Self::Complete(complete) => Ok(complete.to_object(py)),
            Self::Mux(mux) => Ok(mux.to_object(py)),
            Self::Concat(concat) => Ok(concat.to_object(py)),
        }
    }

    pub fn get_signature(&self, py: Python<'py>) -> PyResult<IntoSignature> {
        match self {
            Self::Signature(signature) => Ok(signature.clone()),
            _ => {
                let error_ty = self.to_object(py)?;
                Err(PyTypeError::new_err(format!(
                    r#""{error_ty}" is not allowed"#
                )))
            }
        }
    }

    fn connect_part(
        component: Borrowed<'_, 'py, PyComponent>,
        source: &IntoSignature,
        sink: &Bound<'py, PySignature>,
        component_index: &mut usize,
        pin_index: &mut usize,
    ) -> PyResult<()> {
        let py = component.py();
        let source = source.clone().into_signature(py)?;

        let ((component_budget, pin_budget), sink_n_pins) = {
            let sink = sink.borrow();
            (sink.counts(py), sink.pins.len(py))
        };

        let source = source.bind(py);

        let ((mut components_to_connect, mut pins_to_connect), source_n_pins) = {
            let source = source.borrow();
            (source.counts(py), source.pins.len(py))
        };

        Ok(())
    }

    pub fn connect(
        self,
        component: Borrowed<'_, 'py, PyComponent>,
        sink: IntoSignature,
    ) -> PyResult<()> {
        let py = component.py();
        let sink = sink.into_signature(py)?;

        let (source, kind) = match self {
            Connector::Signature(signature) => (signature.into_signature(py)?, None),
            Connector::Direct(direct) => (
                direct.borrow().0.clone().into_signature(py)?,
                Some(PyConnectionKind::DIRECT),
            ),
            Connector::Complete(complete) => (
                complete.borrow().0.clone().into_signature(py)?,
                Some(PyConnectionKind::COMPLETE),
            ),
            Connector::Mux(mux) => (
                mux.borrow().0.clone().into_signature(py)?,
                Some(PyConnectionKind::MUX),
            ),
            Connector::Concat(concat) => {
                let sink = sink.bind(py);
                let (mut component_index, mut pin_index) = (0, 0);

                return concat.borrow().0.iter().try_for_each(|source| {
                    Self::connect_part(
                        component.as_borrowed(),
                        source,
                        sink,
                        &mut component_index,
                        &mut pin_index,
                    )
                });
            }
        };

        component
            .borrow_mut()
            .add_connection(source.bind(py), sink.bind(py), kind)
    }
}
