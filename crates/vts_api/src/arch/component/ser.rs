use std::ops::Deref;

use pyo3::prelude::*;
use pyo3::types::PyMapping;
use serde::{
    ser::{self, SerializeMap, SerializeStruct},
    Serialize, Serializer,
};
use vts_core::arch::ComponentClass;

use crate::arch::{map_py_ser_err, port::ser::PyPortsSerializer, PyComponent};

pub struct PyComponentSerializer<'py> {
    py: Python<'py>,
    component: &'py PyComponent,
}

impl<'py> PyComponentSerializer<'py> {
    pub fn new(py: Python<'py>, component: &'py PyComponent) -> Self {
        Self { py, component }
    }
}

impl<'py> Serialize for PyComponentSerializer<'py> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.py;
        let mut component_serializer = serializer.serialize_struct("Component", 4)?;

        let ports = self.component.ports.as_ref(py);
        let ports_serializer = PyPortsSerializer::new(ports.as_mapping());
        component_serializer.serialize_field("ports", &ports_serializer)?;

        // TODO: references
        // component_serializer.serialize_field("references")

        if let Some(class) = self.component.class_ {
            component_serializer.serialize_field("class", &ComponentClass::from(class))?;
        }

        component_serializer.end()
    }
}

pub struct PyComponentsSerializer<'py> {
    py: Python<'py>,
    components: &'py PyMapping,
}

impl<'py> PyComponentsSerializer<'py> {
    pub fn new(py: Python<'py>, components: &'py PyMapping) -> Self {
        Self { py, components }
    }
}

impl<'py> Serialize for PyComponentsSerializer<'py> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.py;
        let n_comps = map_py_ser_err!(self.components.len())?;
        let mut components_serializer = map_py_ser_err!(serializer.serialize_map(Some(n_comps)))?;

        let components = map_py_ser_err!(self.components.items())?;
        let mut iter = map_py_ser_err!(components.iter())?;
        while let Some(item) = map_py_ser_err!(iter.next().transpose())? {
            let (name, component) =
                map_py_ser_err!(PyAny::extract::<(&str, Py<PyComponent>)>(item))?;
            let component = map_py_ser_err!(component.try_borrow(py))?;

            let component_serializer = PyComponentSerializer::new(py, component.deref());
            components_serializer.serialize_entry(name, &component_serializer)?;
        }

        components_serializer.end()
    }
}
