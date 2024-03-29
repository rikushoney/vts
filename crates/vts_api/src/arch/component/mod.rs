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
    pub fn new(
        py: Python<'_>,
        name: &Bound<'_, PyString>,
        class_: Option<PyComponentClass>,
    ) -> PyResult<Self> {
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
        let mut component = PyComponent::new(py, &name, self.class_)?;

        let ports = self.ports.bind(py);
        iter_dict_items!(for (name: &str [extract], port: PyPort [downcast]) in ports => {
            component.add_port(py, name, port)?;
        });

        let references = self.references.bind(py);
        iter_dict_items!(for (alias: &str [extract], reference: PyComponentRef [downcast]) in references => {
            let reference = reference.borrow();
            let reference = Bound::new(py, reference.component.clone())?;
            component.add_reference(py, &reference, Some(alias))?;
        });

        Ok(component)
    }

    pub fn add_reference(
        &mut self,
        py: Python<'_>,
        component: &Bound<'_, PyComponent>,
        alias: Option<&str>,
    ) -> PyResult<Py<PyComponentRef>> {
        let alias = match alias {
            Some(alias) => PyString::new_bound(py, alias),
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

        let reference = PyComponentRef::new(py, component, Some(&alias))?;
        let reference = Bound::new(py, reference)?;

        references.deref().set_item(alias, reference.clone())?;

        Ok(reference.unbind())
    }

    pub fn add_port(
        &mut self,
        py: Python<'_>,
        name: &str,
        port: &Bound<'_, PyPort>,
    ) -> PyResult<Py<PyPort>> {
        let ports = self.ports.bind(py);
        let name = PyString::new_bound(py, name);

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

    pub fn add_ports(&mut self, py: Python<'_>, ports: &Bound<'_, PyMapping>) -> PyResult<()> {
        iter_mapping_items!(for (name: &str [extract], port: PyPort [downcast]) in ports => {
            self.add_port(py, name, port)?;
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
        _py: Python<'_>,
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<PyString>>,
    ) -> PyResult<Self> {
        let component = component.clone().unbind();
        let alias = alias.map(|alias| alias.clone().unbind());
        Ok(Self { component, alias })
    }
}
