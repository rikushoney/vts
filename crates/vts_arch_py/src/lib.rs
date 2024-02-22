use pyo3::prelude::*;

extern crate vts_arch as vts_arch_rs;

#[pyclass]
pub struct PyCell {
    _cell: vts_arch_rs::Cell,
}

#[pymethods]
impl PyCell {
    #[new]
    pub fn __init__() -> Self {
        todo!()
    }

    pub fn add_port(&mut self, _name: &str, _port: &PyPort) {
        todo!()
    }
}

#[pyclass]
pub struct PyPort {
    _port: vts_arch_rs::Port,
}

#[pymodule]
fn vts_arch(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyCell>()?;
    m.add_class::<PyPort>()?;
    Ok(())
}
