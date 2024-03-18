pub mod ser;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};

use crate::arch::PyComponent;

#[pyclass]
#[pyo3(name = "PyModule")]
pub struct PyModule_ {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub components: Py<PyDict>,
}

#[pymethods]
impl PyModule_ {
    #[new]
    pub fn new(py: Python<'_>, name: &str) -> Self {
        Self {
            name: PyString::new(py, name).into_py(py),
            components: PyDict::new(py).into_py(py),
        }
    }

    pub fn add_component(
        &mut self,
        py: Python<'_>,
        name: &str,
        component: Py<PyComponent>,
    ) -> PyResult<Py<PyComponent>> {
        let components = self.components.as_ref(py);
        let name = PyString::new(py, name);

        if components.contains(name)? {
            let component_name = name.to_str()?;
            let module_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"component with name "{component_name}" already in "{module_name}""#
            )));
        }

        let component = component.as_ref(py).try_borrow()?;
        let component = Py::new(py, component.copy(py)?)?;

        components.set_item(name, component.clone_ref(py))?;
        Ok(component)
    }

    pub fn add_components(&mut self, py: Python<'_>, components: &PyMapping) -> PyResult<()> {
        for item in components.items()?.iter()? {
            let (name, component) = PyAny::extract::<(&str, Py<PyComponent>)>(item?)?;
            self.add_component(py, name, component)?;
        }
        Ok(())
    }
}
