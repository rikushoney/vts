use pyo3::prelude::*;
use serde::{
    ser::{self, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{component::ser::PyComponentsSerializer, PyModule as PyModule_};

pub struct PyModuleSerializer<'py> {
    module: Bound<'py, PyModule_>,
}

impl<'py> PyModuleSerializer<'py> {
    pub fn new(module: Bound<'py, PyModule_>) -> Self {
        Self { module }
    }
}

impl<'py> Serialize for PyModuleSerializer<'py> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.module.py();
        let mut module_serializer = serializer.serialize_struct("Module", 2)?;

        let module = self.module.borrow();
        let name = map_py_ser_err!(module.name.to_str(py))?;
        module_serializer.serialize_field("name", name)?;

        let components = module.components.bind(py);
        let components_serializer = PyComponentsSerializer::new(components.clone());
        module_serializer.serialize_field("components", &components_serializer)?;

        module_serializer.end()
    }
}
