use pyo3::{
    exceptions::{PyAttributeError, PyTypeError},
    prelude::*,
    types::{PyString, PyTuple},
};
use vts_core::arch::{
    component::ComponentKey,
    connection::{ComponentRefSelection, ConnectionKind},
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
pub struct PyComponentRefPort(
    pub(crate) Py<PyComponentRefSelection>,
    pub(crate) Py<PyPort>,
);

#[pymethods]
impl PyComponentRefPort {
    pub fn __getitem__(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PySignature> {
        let port = self.1.bind(py).borrow();
        let pins = port.select(py, index)?;

        Ok(PySignature(
            pins,
            ComponentOrRef::Reference(self.0.borrow(py).clone()),
        ))
    }

    pub fn __setitem__(
        &self,
        py: Python<'_>,
        sink: SliceOrIndex<'_>,
        source: Connector<'_>,
    ) -> PyResult<()> {
        let sink = Bound::new(py, self.__getitem__(py, sink)?)?;
        let selection = self.0.bind(py).borrow();
        let reference = selection.0.bind(py).borrow();

        source.connect(
            reference.parent(py)?.as_borrowed(),
            IntoSignature::Signature(sink.unbind()),
        )
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
    pub fn get_reference(&self, py: Python<'_>) -> Option<ComponentRefSelection> {
        match &self.1 {
            ComponentOrRef::Reference(selection) => Some(ComponentRefSelection::new(
                selection.0.bind(py).borrow().key(),
                selection.1.clone(),
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
#[pyclass(name = "ComponentRefSelection")]
pub struct PyComponentRefSelection(pub(super) Py<PyComponentRef>, pub(super) ReferenceRange);

#[pymethods]
impl PyComponentRefSelection {
    pub fn __getattr__(&self, port: &Bound<'_, PyString>) -> PyResult<PyComponentRefPort> {
        let py = port.py();
        let reference = self.0.bind(py);

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
                .key()
        };

        let port = PyPort::new(reference.borrow().module(py).as_borrowed(), port)?;
        let reference = PyComponentRef::__getitem__(reference, py, SliceOrIndex::full(py))?;
        Ok(PyComponentRefPort(Py::new(py, reference)?, port.unbind()))
    }

    pub fn __setattr__(&self, sink: &Bound<'_, PyString>, source: Connector<'_>) -> PyResult<()> {
        let py = sink.py();

        let sink = {
            let sink = self.__getattr__(sink)?;
            Bound::new(py, sink.__getitem__(py, SliceOrIndex::full(py))?)?
        };

        let reference = self.0.bind(py).borrow();
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

    pub fn connect(
        self,
        component: Borrowed<'_, 'py, PyComponent>,
        sink: IntoSignature,
    ) -> PyResult<()> {
        let py = component.py();

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
                #[allow(unused_mut)]
                let mut component = component.borrow_mut();
                return concat.borrow().0.iter().try_for_each(|signature| {
                    // TODO: count pins and make connections appropriately
                    let _ = component;
                    let _ = signature;
                    todo!()
                });
            }
        };

        component.borrow_mut().add_connection(
            source.bind(py),
            sink.into_signature(py)?.bind(py),
            kind,
        )
    }
}
