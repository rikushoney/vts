use std::ops::Deref;

use pyo3::prelude::*;
use pyo3::types::PyMapping;
use serde::{
    ser::{self, SerializeMap, SerializeStruct},
    Serialize, Serializer,
};
use vts_core::arch::{PortClass, PortKind};

use crate::arch::{map_py_ser_err, PyPort};

impl Serialize for PyPort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut port_serializer = serializer.serialize_struct("Port", 4)?;

        port_serializer.serialize_field("kind", &PortKind::from(self.kind))?;
        port_serializer.serialize_field("n_pins", &self.n_pins)?;

        if let Some(class) = self.class_ {
            port_serializer.serialize_field("class", &PortClass::from(class))?;
        }

        port_serializer.end()
    }
}

pub struct PyPortsSerializer<'py> {
    ports: &'py PyMapping,
}

impl<'py> PyPortsSerializer<'py> {
    pub fn new(ports: &'py PyMapping) -> Self {
        Self { ports }
    }
}

impl<'py> Serialize for PyPortsSerializer<'py> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.ports.py();
        let n_ports = map_py_ser_err!(self.ports.len())?;
        let mut ports_serializer = map_py_ser_err!(serializer.serialize_map(Some(n_ports)))?;

        for item in map_py_ser_err!(self.ports.iter())? {
            let (name, port) =
                map_py_ser_err!(PyAny::extract::<(&str, Py<PyPort>)>(map_py_ser_err!(item)?))?;
            let port = map_py_ser_err!(port.try_borrow(py))?;
            ports_serializer.serialize_entry(name, port.deref())?;
        }

        ports_serializer.end()
    }
}
