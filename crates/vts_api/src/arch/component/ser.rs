use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{
    ser::{self, SerializeMap, SerializeStruct},
    Serialize, Serializer,
};
use vts_core::arch::ComponentClass;

use crate::arch::{map_py_ser_err, port::ser::PyPortsSerializer, PyComponent};

pub struct PyComponentSerializer<'py> {
    component: Bound<'py, PyComponent>,
}

impl<'py> PyComponentSerializer<'py> {
    pub fn new(component: Bound<'py, PyComponent>) -> Self {
        Self { component }
    }
}

impl<'py> Serialize for PyComponentSerializer<'py> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.component.py();
        let mut component_serializer = serializer.serialize_struct("Component", 4)?;

        let component = self.component.borrow();
        let ports = component.ports.bind(py);
        let ports_serializer = PyPortsSerializer::new(ports.clone());
        component_serializer.serialize_field("ports", &ports_serializer)?;

        // TODO: references
        // component_serializer.serialize_field("references")

        if let Some(class) = component.class_ {
            component_serializer.serialize_field("class", &ComponentClass::from(class))?;
        }

        component_serializer.end()
    }
}

pub struct PyComponentsSerializer<'py> {
    components: Bound<'py, PyDict>,
}

impl<'py> PyComponentsSerializer<'py> {
    pub fn new(components: Bound<'py, PyDict>) -> Self {
        Self { components }
    }
}

impl<'py> Serialize for PyComponentsSerializer<'py> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let n_comps = self.components.len();
        let mut components_serializer = map_py_ser_err!(serializer.serialize_map(Some(n_comps)))?;

        for (name, component) in self.components.iter() {
            let name = map_py_ser_err!(name.extract::<&str>())?;
            let component = map_py_ser_err!(component.downcast::<PyComponent>())?;
            let component_serializer = PyComponentSerializer::new(component.clone());

            components_serializer.serialize_entry(name, &component_serializer)?;
        }

        components_serializer.end()
    }
}
