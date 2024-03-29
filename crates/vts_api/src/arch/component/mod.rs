pub mod de;
pub mod ser;

use std::ops::Deref;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_core::arch::ComponentClass;

use crate::arch::{iter_dict_items, iter_mapping_items, PyPort};

wrap_enum!(PyComponentClass => ComponentClass:
    LUT = Lut,
    LATCH = Latch,
);

#[pyclass]
#[derive(Clone)]
pub struct PyComponent {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub ports: Py<PyDict>,
    #[pyo3(get, set)]
    pub references: Py<PyDict>,
    #[pyo3(get, set)]
    pub class_: Option<PyComponentClass>,
}

#[pymethods]
impl PyComponent {
    #[new]
    pub fn new(name: &Bound<'_, PyString>, class_: Option<PyComponentClass>) -> PyResult<Self> {
        let py = name.py();

        let name = name.clone().unbind();
        let ports = PyDict::new_bound(py).into();
        let references = PyDict::new_bound(py).into();

        Ok(Self {
            name,
            ports,
            references,
            class_,
        })
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let name = PyString::new_bound(py, self.name.bind(py).to_str()?);
        let mut component = PyComponent::new(&name, self.class_)?;

        let ports = self.ports.bind(py);
        iter_dict_items!(for (name: PyString, port: PyPort) in ports => {
            component.add_port(name, port)?;
        });

        let references = self.references.bind(py);
        iter_dict_items!(for (alias: PyString, reference: PyComponentRef) in references => {
            let reference = reference.borrow();
            let reference = Bound::new(py, reference.component.clone())?;
            component.add_reference(&reference, Some(alias))?;
        });

        Ok(component)
    }

    pub fn add_reference(
        &mut self,
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<'_, PyString>>,
    ) -> PyResult<Py<PyComponentRef>> {
        let py = component.py();

        let alias = match alias {
            Some(alias) => alias.clone(),
            None => {
                let component = component.borrow();
                let alias = component.name.bind(py);
                PyString::new_bound(py, alias.to_str()?)
            }
        };

        let references = self.references.bind(py);
        if references.deref().contains(alias.clone())? {
            return Err(PyValueError::new_err(format!(
                r#"component or alias "{alias}" already referenced in "{component}""#,
                alias = alias.to_str()?,
                component = self.name.bind(py).to_str()?
            )));
        }

        let reference = PyComponentRef::new(component, Some(&alias))?;
        let reference = Bound::new(py, reference)?;

        references.deref().set_item(alias, reference.clone())?;

        Ok(reference.unbind())
    }

    pub fn add_port(
        &mut self,
        name: &Bound<'_, PyString>,
        port: &Bound<'_, PyPort>,
    ) -> PyResult<Py<PyPort>> {
        let py = name.py();

        let ports = self.ports.bind(py);

        if ports.contains(name.clone())? {
            return Err(PyValueError::new_err(format!(
                r#"port "{port}" already in "{component}""#,
                port = name.to_str()?,
                component = self.name.bind(py).to_str()?,
            )));
        }

        let port = port.borrow();
        let port = Py::new(py, port.copy(py)?)?;

        ports.set_item(name, port.clone_ref(py))?;

        Ok(port)
    }

    pub fn add_ports(&mut self, ports: &Bound<'_, PyMapping>) -> PyResult<()> {
        iter_mapping_items!(for (name: PyString, port: PyPort) in ports => {
            self.add_port(name, port)?;
        });

        Ok(())
    }
}

#[pyclass]
pub struct PyComponentRef {
    #[pyo3(get, set)]
    pub component: Py<PyComponent>,
    #[pyo3(get, set)]
    pub alias: Option<Py<PyString>>,
}

#[pymethods]
impl PyComponentRef {
    #[new]
    pub fn new(
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<PyString>>,
    ) -> PyResult<Self> {
        let component = component.clone().unbind();
        let alias = alias.map(|alias| alias.clone().unbind());
        Ok(Self { component, alias })
    }
}
