use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::PyString;

use vts_core::arch1::{
    connection::ComponentRefs, module::ComponentRefId, prelude::*, reference::ReferenceRange,
};

use super::{
    connection::{Connector, IntoSignature},
    prelude::*,
};

#[pyclass(name = "ComponentRef")]
pub struct PyComponentRef(Py<PyModule_>, ComponentRefId);

impl PyComponentRef {
    pub(crate) fn new<'py>(
        module: Borrowed<'_, 'py, PyModule_>,
        reference: ComponentRefId,
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

    pub(crate) fn id(&self) -> ComponentRefId {
        self.1
    }
}

pub(crate) trait PyComponentRefMethodsImpl {
    fn try_with_inner<F, T>(&self, f: F) -> PyResult<T>
    where
        F: FnMut(ComponentRef<'_>) -> PyResult<T>;
}

impl PyComponentRefMethodsImpl for Bound<'_, PyComponentRef> {
    fn try_with_inner<F, T>(&self, mut f: F) -> PyResult<T>
    where
        F: FnMut(ComponentRef<'_>) -> PyResult<T>,
    {
        let py = self.py();
        let reference = self.borrow();
        borrow_inner!(reference + py => reference);
        f(reference)
    }
}

pub trait PyComponentRefMethods {
    fn select(&self, py: Python<'_>, range: ReferenceRange) -> ComponentRefs;

    fn n_instances(&self, py: Python<'_>) -> u32;

    fn select_py(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PyComponentRefs>;
}

impl PyComponentRefMethods for Bound<'_, PyComponentRef> {
    fn select(&self, py: Python<'_>, range: ReferenceRange) -> ComponentRefs {
        let reference = self.borrow();
        borrow_inner!(reference + py => reference);
        reference.select(range)
    }

    fn n_instances(&self, py: Python<'_>) -> u32 {
        self.borrow().n_instances(py)
    }

    fn select_py(&self, py: Python<'_>, index: SliceOrIndex<'_>) -> PyResult<PyComponentRefs> {
        PyComponentRef::__getitem__(self, py, index)
    }
}

macro_rules! borrow_inner {
    ($slf:ident + $py:ident => $ref:ident) => {
        let module = $slf.module($py).borrow();
        let inner = module.inner.borrow($py);
        let $ref = inner
            .0
            .get_reference($slf.id())
            .expect("reference should be in module");
    };
}

pub(super) use borrow_inner;

#[pymethods]
impl PyComponentRef {
    pub fn module<'py>(&self, py: Python<'py>) -> &Bound<'py, PyModule_> {
        self.0.bind(py)
    }

    pub fn component<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => reference);
        PyComponent::new(self.module(py).into(), reference.component().unbind())
    }

    pub fn parent<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyComponent>> {
        borrow_inner!(self + py => reference);
        PyComponent::new(self.module(py).into(), reference.parent().unbind())
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

    #[pyo3(name = "select")]
    pub fn select_py(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        index: SliceOrIndex<'_>,
    ) -> PyResult<PyComponentRefs> {
        let n_instances = slf.n_instances(py);
        let mut range = ReferenceRange::Bound(index.to_range(n_instances)?);
        range.flatten(n_instances);
        Ok(PyComponentRefs::new(slf.as_borrowed(), range))
    }

    pub fn __getitem__(
        slf: &Bound<'_, Self>,
        py: Python<'_>,
        index: SliceOrIndex<'_>,
    ) -> PyResult<PyComponentRefs> {
        Self::select_py(slf, py, index)
    }

    pub fn __getattr__(
        slf: &Bound<'_, Self>,
        port: &Bound<'_, PyString>,
    ) -> PyResult<PyComponentRefPort> {
        let py = slf.py();

        let port = slf.try_with_inner(|reference| {
            Ok(reference
                .component()
                .find_port(port.to_str()?)
                .ok_or(PyAttributeError::new_err(format!(
                    r#"undefined port "{port}" referenced in "{component}""#,
                    component = reference.component().name()
                )))?
                .unbind())
        })?;

        let reference = Py::new(py, Self::select_py(slf, py, SliceOrIndex::full(py))?)?;
        let port = PyPort::new(slf.borrow().module(py).as_borrowed(), port)?;

        Ok(PyComponentRefPort::new(
            reference.bind_borrowed(py),
            port.as_borrowed(),
        ))
    }

    pub fn __setattr__(
        slf: &Bound<'_, Self>,
        sink: &Bound<'_, PyString>,
        source: Connector<'_>,
    ) -> PyResult<()> {
        let py = slf.py();
        let sink = Self::__getattr__(slf, sink)?;
        let sink = sink.__getitem__(py, SliceOrIndex::full(py))?;
        let reference = slf.borrow();
        let parent = reference.parent(py)?;

        source.connect(
            parent.as_borrowed(),
            IntoSignature::Signature(sink.unbind()),
        )
    }
}
