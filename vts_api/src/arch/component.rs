#![allow(unused)] // TODO: remove this!

use std::ops::Deref;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyMapping, PyString};
use vts_core::arch::{
    module::{ComponentId, ComponentRefId},
    Component, ComponentClass,
};

use super::{module::PyModule_, port::PyPortPins, PyPort};

wrap_enum!(PyComponentClass => ComponentClass:
    LUT = Lut,
    LATCH = Latch,
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyComponent(pub(crate) Py<PyModule_>, pub(crate) ComponentId);

#[pymethods]
impl PyComponent {
    pub fn add_port(
        &mut self,
        name: &Bound<'_, PyString>,
        port: &Bound<'_, PyPort>,
    ) -> PyResult<Py<PyPort>> {
        // let py = name.py();

        // let ports = self.ports.bind(py);

        // if ports.contains(name.clone())? {
        //     return Err(PyValueError::new_err(format!(
        //         r#"port "{port}" already in "{component}""#,
        //         port = name.to_str()?,
        //         component = self.name.bind(py).to_str()?,
        //     )));
        // }

        // let port = port.borrow();
        // let port = Py::new(py, port.copy(py)?)?;

        // ports.set_item(name, port.clone_ref(py))?;

        // Ok(port)
        todo!()
    }

    pub fn add_ports(&mut self, ports: &Bound<'_, PyMapping>) -> PyResult<()> {
        // iter_mapping_items!(for (name: PyString, port: PyPort) in ports => {
        //     self.add_port(name, port)?;
        // });

        Ok(())
    }

    pub fn add_reference(
        &mut self,
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<'_, PyString>>,
        n_instances: Option<usize>,
    ) -> PyResult<Py<PyComponentRef>> {
        // let py = component.py();

        // let alias = match alias {
        //     Some(alias) => alias.clone(),
        //     None => {
        //         let component = component.borrow();
        //         let alias = component.name.bind(py);
        //         PyString::new_bound(py, alias.to_str()?)
        //     }
        // };

        // let references = self.references.bind(py);
        // if references.deref().contains(alias.clone())? {
        //     return Err(PyValueError::new_err(format!(
        //         r#"component or alias "{alias}" already referenced in "{component}""#,
        //         alias = alias.to_str()?,
        //         component = self.name.bind(py).to_str()?
        //     )));
        // }

        // let reference = PyComponentRef::new(component, Some(&alias), n_instances);
        // let reference = Bound::new(py, reference)?;

        // references.deref().set_item(alias, reference.clone())?;

        // Ok(reference.unbind())
        todo!()
    }

    pub fn add_connection(
        &mut self,
        source_pins: &Bound<'_, PyPortPins>,
        sink_pins: &Bound<'_, PyPortPins>,
        source_component: Option<&Bound<'_, PyComponentRef>>,
        sink_component: Option<&Bound<'_, PyComponentRef>>,
    ) -> PyResult<Py<PyConnection>> {
        // let py = source_pins.py();

        // // TODO: check for duplicate connections?

        // let connections = self.connections.bind(py);
        // let connection = PyConnection::new(
        //     source_pins.clone(),
        //     sink_pins.clone(),
        //     source_component.cloned(),
        //     sink_component.cloned(),
        // );
        // let connection = Bound::new(py, connection)?;

        // connections.append(connection.clone())?;

        // Ok(connection.unbind())
        todo!()
    }
}

#[pyclass]
pub struct PyComponentRef(Py<PyModule_>, ComponentRefId);

#[pymethods]
impl PyComponentRef {}

#[pyclass]
pub struct PyConnection {}

#[pymethods]
impl PyConnection {}
