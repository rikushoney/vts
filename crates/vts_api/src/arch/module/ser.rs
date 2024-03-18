use pyo3::prelude::*;
use serde::{
    ser::{self, SerializeStruct},
    Serialize, Serializer,
};

use crate::arch::{component::ser::PyComponentsSerializer, map_py_ser_err, PyModule as PyModule_};

pub struct PyModuleSerializer<'py> {
    py: Python<'py>,
    module: &'py PyModule_,
}

impl<'py> Serialize for PyModuleSerializer<'py> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.py;
        let mut module_serializer = serializer.serialize_struct("Module", 2)?;

        let name = map_py_ser_err!(self.module.name.as_ref(py).to_str())?;
        module_serializer.serialize_field("name", name)?;

        let components = self.module.components.as_ref(py);
        let components_serializer = PyComponentsSerializer::new(py, components.as_mapping());
        module_serializer.serialize_field("components", &components_serializer)?;

        module_serializer.end()
    }
}
