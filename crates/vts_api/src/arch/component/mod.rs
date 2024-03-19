pub mod de;
pub mod ser;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_core::arch::ComponentClass;

use crate::arch::PyPort;

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
        name: Py<PyString>,
        class_: Option<PyComponentClass>,
    ) -> PyResult<Self> {
        Ok(Self {
            name,
            ports: PyDict::new(py).into(),
            references: PyDict::new(py).into(),
            class_,
        })
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let name = PyString::new(py, self.name.as_ref(py).to_str()?);
        let mut component = PyComponent::new(py, name.into_py(py), self.class_)?;

        for item in self.ports.as_ref(py).items().iter() {
            let (name, port) = PyAny::extract::<(&str, Py<PyPort>)>(item)?;
            component.add_port(py, name, port)?;
        }

        for item in self.references.as_ref(py).items().iter() {
            let (name, reference) = PyAny::extract::<(&str, Py<PyComponent>)>(item)?;
            component.add_ref(py, name, reference)?;
        }

        Ok(component)
    }

    pub fn add_ref(
        &mut self,
        py: Python<'_>,
        name: &str,
        component: Py<PyComponent>,
    ) -> PyResult<Py<PyComponent>> {
        let references = self.references.as_ref(py);
        let name = PyString::new(py, name);

        if references.contains(name)? {
            let reference_name = name.to_str()?;
            let component_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"component with name "{reference_name}" already referenced in "{component_name}""#
            )));
        }

        let component = component.as_ref(py).try_borrow()?;
        let component = Py::new(py, component.copy(py)?)?;

        references.set_item(name, component.clone_ref(py))?;
        Ok(component)
    }

    pub fn add_port(
        &mut self,
        py: Python<'_>,
        name: &str,
        port: Py<PyPort>,
    ) -> PyResult<Py<PyPort>> {
        let ports = self.ports.as_ref(py);
        let name = PyString::new(py, name);

        if ports.contains(name)? {
            let port_name = name.to_str()?;
            let module_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"port with name "{port_name}" already in "{module_name}""#
            )));
        }

        let port = port.as_ref(py).try_borrow()?;
        let port = Py::new(py, port.copy(py)?)?;

        ports.set_item(name, port.clone_ref(py))?;
        Ok(port)
    }

    pub fn add_ports(&mut self, py: Python<'_>, ports: &PyMapping) -> PyResult<()> {
        for item in ports.items()?.iter()? {
            let (name, port) = PyAny::extract::<(&str, Py<PyPort>)>(item?)?;
            self.add_port(py, name, port)?;
        }
        Ok(())
    }
}
