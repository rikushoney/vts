use hashbrown::HashMap;
use pyo3::prelude::*;

use std::sync::Arc;

#[pyclass]
struct Block {
    name: Arc<str>,
    ports: HashMap<Arc<str>, Port>,
}

#[pymethods]
impl Block {
    #[new]
    fn new(name: &str) -> Self {
        let name = name.into();
        Self {
            name,
            ports: HashMap::new(),
        }
    }

    #[getter]
    fn name(&self) -> &str {
        &self.name
    }
}

#[pyclass]
struct Port {
    dummy: usize,
}

#[pymodule]
fn vts_arch(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Block>()?;
    m.add_class::<Port>()?;
    Ok(())
}
