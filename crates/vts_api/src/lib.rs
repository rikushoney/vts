use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pyclass]
#[derive(Clone)]
pub struct PyComponent {
    ports: Py<PyDict>,
}

#[pymethods]
impl PyComponent {
    #[new]
    pub fn __init__(py: Python<'_>) -> Self {
        let ports = PyDict::new(py).into();
        Self { ports }
    }

    pub fn add_port(&mut self, py: Python<'_>, name: &str, port: PyPort) -> PyResult<()> {
        self.ports.as_ref(py).set_item(name, port.into_py(py))?;
        Ok(())
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyPort {}

#[pymethods]
impl PyPort {
    #[new]
    pub fn __init__() -> Self {
        Self {}
    }
}

#[pymodule]
fn vts_api_rs(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyComponent>()?;
    m.add_class::<PyPort>()?;
    Ok(())
}
