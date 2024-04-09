use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::PyString;

use vts_core::arch::component::ComponentRefKey;

use super::component::{PyComponent, PyConnectionKind};
use super::module::PyModule_;
use super::port::{PyComponentRefPort, PyPort, PyPortSelection, SliceOrIndex};

#[pyclass]
pub struct PyComponentRef(Py<PyModule_>, ComponentRefKey);

impl PyComponentRef {
    pub(crate) fn new<'py>(
        py: Python<'py>,
        module: &Bound<'py, PyModule_>,
        reference: ComponentRefKey,
    ) -> PyResult<Bound<'py, Self>> {
        if let Some(reference) = module.borrow().references.get(&reference) {
            Ok(reference.bind(py).clone())
        } else {
            let py_reference = Py::new(py, Self(module.clone().unbind(), reference))?;

            module
                .borrow_mut()
                .references
                .insert(reference, py_reference.clone());

            Ok(py_reference.bind(py).clone())
        }
    }

    pub(crate) fn key(&self) -> ComponentRefKey {
        self.1
    }
}

macro_rules! get_reference {
    ($slf:ident + $py:ident => $ref:ident) => {
        let module = $slf.module($py).borrow();
        let $ref = module
            .inner
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
        get_reference!(self + py => reference);
        PyComponent::new(py, self.module(py), reference.component().key())
    }

    pub fn alias<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyString>> {
        get_reference!(self + py => reference);

        if let Some(alias) = reference.alias() {
            Some(PyString::new_bound(py, alias))
        } else {
            None
        }
    }

    pub fn alias_or_name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        get_reference!(self + py => reference);
        PyString::new_bound(py, reference.alias_or_name())
    }

    pub fn n_instances(&self, py: Python<'_>) -> usize {
        get_reference!(self + py => reference);
        reference.n_instances()
    }

    pub fn __getattr__(
        slf: &Bound<'_, PyComponentRef>,
        py: Python<'_>,
        port: &Bound<'_, PyString>,
    ) -> PyResult<PyComponentRefPort> {
        let reference = slf.borrow();
        get_reference!(reference + py => reference);
        let port = port.to_str()?;

        if let Some(port) = reference.component().find_port(port) {
            let port = PyPort::new(py, slf.borrow().module(py), port.key())?;
            Ok(PyComponentRefPort(slf.clone().unbind(), port.unbind()))
        } else {
            let component = reference.component().name();
            Err(PyAttributeError::new_err(format!(
                r#"undefined port "{port}" referenced in "{component}""#
            )))
        }
    }

    pub fn __setattr__(
        slf: &Bound<'_, PyComponentRef>,
        py: Python<'_>,
        source: &Bound<'_, PyString>,
        sink: &Bound<'_, PyPortSelection>,
    ) -> PyResult<()> {
        let port = Self::__getattr__(slf, py, source)?;
        let source = {
            let source = port.__getitem__(py, SliceOrIndex::full(py))?;
            Bound::new(py, source)?
        };

        slf.borrow().component(py)?.borrow_mut().add_connection(
            py,
            &source,
            sink,
            Some(PyConnectionKind::DIRECT),
        )
    }
}
