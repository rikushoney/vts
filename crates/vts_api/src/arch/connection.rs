use pyo3::{
    exceptions::{PyAttributeError, PyTypeError},
    prelude::*,
    types::{PyString, PyTuple},
};
use vts_core::arch1::{
    builder::prelude::*,
    connection::{ComponentRefs, Concat},
    prelude::*,
    reference::ReferenceRange,
};

use super::{module, prelude::*, reference};

wrap_enum!(
    PyConnectionKind (name = "ConnectionKind", help = "connection kind") => ConnectionKind:
        DIRECT = Direct (alias = "direct" | "d"),
        COMPLETE = Complete (alias = "complete" | "c"),
        MUX = Mux (alias = "mux" | "m")
);

#[pyclass(name = "ComponentRefPort")]
#[derive(Clone, Debug)]
pub struct PyComponentRefPort {
    components: Py<PyComponentRefs>,
    port: Py<PyPort>,
}

impl PyComponentRefPort {
    pub fn new(
        components: Borrowed<'_, '_, PyComponentRefs>,
        port: Borrowed<'_, '_, PyPort>,
    ) -> Self {
        Self {
            components: components.to_owned().unbind(),
            port: port.to_owned().unbind(),
        }
    }

    pub fn components<'py>(&self, py: Python<'py>) -> &Bound<'py, PyComponentRefs> {
        self.components.bind(py)
    }

    pub fn port<'py>(&self, py: Python<'py>) -> &Bound<'py, PyPort> {
        self.port.bind(py)
    }
}

#[pymethods]
impl PyComponentRefPort {
    pub fn __getitem__<'py>(
        &self,
        py: Python<'py>,
        index: SliceOrIndex<'_>,
    ) -> PyResult<Bound<'py, PySignature>> {
        let port = self.port.bind(py).borrow();
        let pins = port.select_py(py, index)?;

        let signature = PySignature {
            component_or_reference: ComponentOrRef::Reference(self.components.borrow(py).clone()),
            pins,
        };

        Bound::new(py, signature)
    }

    pub fn __setitem__(
        &self,
        py: Python<'_>,
        sink: SliceOrIndex<'_>,
        source: Connector<'_>,
    ) -> PyResult<()> {
        let sink = self.__getitem__(py, sink)?;
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
    Reference(PyComponentRefs),
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

    pub fn new_reference(reference: PyComponentRefs, pins: PyPortPins) -> Self {
        Self {
            component_or_reference: ComponentOrRef::Reference(reference),
            pins,
        }
    }

    pub fn get_reference(&self, py: Python<'_>) -> Option<ComponentRefs> {
        match &self.component_or_reference {
            ComponentOrRef::Reference(selection) => Some({
                let reference = selection.reference(py).borrow();
                reference::borrow_inner!(reference + py => reference);
                reference.select(selection.range.clone())
            }),
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
#[pyclass(name = "ComponentRefs")]
pub struct PyComponentRefs {
    reference: Py<PyComponentRef>,
    pub range: ReferenceRange,
}

impl PyComponentRefs {
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
impl PyComponentRefs {
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
        let reference = reference.select_py(py, SliceOrIndex::full(py))?;

        Ok(PyComponentRefPort {
            components: Py::new(py, reference)?,
            port: port.unbind(),
        })
    }

    pub fn __setattr__(&self, sink: &Bound<'_, PyString>, source: Connector<'_>) -> PyResult<()> {
        let py = sink.py();

        let sink = self
            .__getattr__(sink)?
            .__getitem__(py, SliceOrIndex::full(py))?;

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
            Self::PortRef(reference) => Ok(reference
                .borrow(py)
                .__getitem__(py, SliceOrIndex::full(py))?
                .unbind()),
            Self::Port(port) => {
                Py::new(py, port.borrow(py).__getitem__(py, SliceOrIndex::full(py))?)
            }
        }
    }
}

pub trait ToSignature {
    fn to_signature(&self, py: Python<'_>) -> PyResult<Py<PySignature>>;
}

impl ToSignature for IntoSignature {
    fn to_signature(&self, py: Python<'_>) -> PyResult<Py<PySignature>> {
        self.clone().into_signature(py)
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
    let extract_signature = |mut signatures: Vec<IntoSignature>, signature: Bound<'_, PyAny>| {
        signature.extract::<IntoSignature>().map(|signature| {
            signatures.push(signature);
            signatures
        })
    };

    connectors
        .iter()
        .try_fold(Vec::new(), extract_signature)
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
                let component = component.borrow();
                let module = component.module(py);
                let sink = sink.bind(py);
                let concat = concat.borrow();
                let mut parts = concat.0.iter();

                let mut concat = {
                    let sink = sink.borrow();

                    match sink.component_or_reference {
                        ComponentOrRef::Component(component) => {
                            Concat::new_component(component, sink.pins.1.clone())
                        }
                        ComponentOrRef::Reference(ref reference) => {
                            let range = reference.range.clone();
                            let reference = reference.reference.bind(py);
                            Concat::new_reference(reference.select(py, range), sink.pins.1.clone())
                        }
                    }
                };

                parts.try_for_each(|signature| {
                    signature.to_signature(py).map(|signature| {
                        module::borrow_inner!(module + py => inner);
                        let source = signature.borrow(py);

                        match source.component_or_reference {
                            ComponentOrRef::Component(component) => {
                                concat.append_component(&inner.0, component, source.pins.1.clone());
                            }
                            ComponentOrRef::Reference(ref reference) => {
                                let range = reference.range.clone();
                                let reference = reference.reference.bind(py);

                                concat.append_reference(
                                    &inner.0,
                                    reference.select(py, range),
                                    source.pins.1.clone(),
                                );
                            }
                        }
                    })
                })?;

                module::borrow_inner_mut!(module + py => inner + checker);
                concat
                    .make_connections(&mut inner.0, &mut checker.0)
                    .map_err(PyLinkerError::from)?;
                return Ok(());
            }
        };

        component
            .borrow()
            .add_connection(source.bind(py), sink.bind(py), kind)
    }
}
