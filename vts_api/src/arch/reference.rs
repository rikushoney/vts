use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::PyString;

use vts_core::arch::reference::{ComponentRefKey, ReferenceRange};

use super::{IntoSignature, PyComponent, PyComponentRefPort, PyModule_, PyPort, SliceOrIndex};

#[pyclass(name = "ComponentRef")]
pub struct PyComponentRef(Py<PyModule_>, ComponentRefKey);

impl PyComponentRef {
    pub(crate) fn new<'py>(
        module: Borrowed<'_, 'py, PyModule_>,
        reference: ComponentRefKey,
    ) -> PyResult<Bound<'py, Self>> {
        let py = module.py();

        if let Some(reference) = module.borrow().references.get(&reference) {
            return Ok(reference.bind(py).clone());
        }

        let py_reference = Py::new(py, Self(module.as_unbound().clone_ref(py), reference))?;

        module
            .borrow_mut()
            .references
            .insert(reference, py_reference.clone());

        Ok(py_reference.bind(py).clone())
    }

    pub(crate) fn key(&self) -> ComponentRefKey {
        self.1
    }
}

macro_rules! borrow_inner {
    ($slf:ident + $py:ident => $ref:ident) => {
        let module = $slf.module($py).borrow();
        let inner = module.inner.borrow($py);
        let $ref = inner
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

    pub fn component<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => reference);
        PyComponent::new(self.module(py).into(), reference.component().key())
    }

    pub fn parent<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => reference);
        PyComponent::new(self.module(py).into(), reference.parent().key())
    }

    pub fn alias<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyString>> {
        borrow_inner!(self + py => reference);

        reference
            .alias()
            .map(|alias| PyString::new_bound(py, alias))
    }

    pub fn alias_or_name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        borrow_inner!(self + py => reference);
        PyString::new_bound(py, reference.alias_or_name())
    }

    pub fn n_instances(&self, py: Python<'_>) -> u32 {
        borrow_inner!(self + py => reference);
        reference.n_instances()
    }

    pub fn select(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        index: SliceOrIndex<'_>,
    ) -> PyResult<PyComponentRefSelection> {
        let n_instances = {
            let slf = slf.borrow();
            slf.n_instances(py)
        };

        let mut range = ReferenceRange::Bound(index.to_range(n_instances)?);
        range.flatten(n_instances);
        Ok(PyComponentRefSelection(slf.clone().unbind(), range))
    }

    pub fn __getitem__(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        index: SliceOrIndex<'_>,
    ) -> PyResult<PyComponentRefSelection> {
        Self::select(slf, py, index)
    }

    pub fn __getattr__(
        slf: &Bound<'_, Self>,
        port: &Bound<'_, PyString>,
    ) -> PyResult<PyComponentRefPort> {
        let py = slf.py();
        let reference = slf.borrow();

        let port = {
            borrow_inner!(reference + py => reference);

            reference
                .component()
                .find_port(port.to_str()?)
                .ok_or(PyAttributeError::new_err(format!(
                    r#"undefined port "{port}" referenced in "{component}""#,
                    component = reference.component().name()
                )))?
                .key()
        };

        let reference = Self::select(slf, py, SliceOrIndex::full(py))?;
        let port = PyPort::new(slf.borrow().module(py).as_borrowed(), port)?;
        Ok(PyComponentRefPort(Py::new(py, reference)?, port.unbind()))
    }

    pub fn __setattr__(
        slf: &Bound<'_, Self>,
        sink: &Bound<'_, PyString>,
        source: IntoSignature<'_>,
    ) -> PyResult<()> {
        let py = slf.py();
        let sink = Self::__getattr__(slf, sink)?;

        let sink = {
            let sink = sink.__getitem__(py, SliceOrIndex::full(py))?;
            Bound::new(py, sink)?
        };

        let reference = slf.borrow();
        let parent = reference.parent(py)?;

        parent
            .borrow_mut()
            .add_connection(&source.into_signature()?, &sink, None)
    }
}

#[derive(Clone, Debug)]
#[pyclass(name = "ComponentRefSelection")]
pub struct PyComponentRefSelection(pub(super) Py<PyComponentRef>, pub(super) ReferenceRange);

#[pymethods]
impl PyComponentRefSelection {
    pub fn __getattr__<'py>(&self, port: &Bound<'py, PyString>) -> PyResult<PyComponentRefPort> {
        let py = port.py();
        let reference = self.0.bind(py);

        let port = {
            let reference = reference.borrow();
            borrow_inner!(reference + py => reference);

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

    pub fn __setattr__<'py>(
        &self,
        sink: &Bound<'py, PyString>,
        source: IntoSignature<'_>,
    ) -> PyResult<()> {
        let py = sink.py();

        let sink = {
            let sink = self.__getattr__(sink)?;
            Bound::new(py, sink.__getitem__(py, SliceOrIndex::full(py))?)?
        };

        let reference = self.0.bind(py).borrow();
        let parent = reference.parent(py)?;

        parent
            .borrow_mut()
            .add_connection(&source.into_signature()?, &sink, None)
    }
}
