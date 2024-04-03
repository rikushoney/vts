use std::ops::Deref;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyMapping, PyString};
use vts_core::arch::ComponentClass;

use crate::arch::{port::PyPortPins, PyPort};

wrap_enum!(PyComponentClass => ComponentClass:
    LUT = Lut,
    LATCH = Latch,
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyComponent {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub ports: Py<PyDict>,
    #[pyo3(get, set)]
    pub references: Py<PyDict>,
    #[pyo3(get, set)]
    pub connections: Py<PyList>,
    #[pyo3(get, set)]
    pub class_: Option<PyComponentClass>,
}

#[pymethods]
impl PyComponent {
    #[new]
    pub fn new(name: &Bound<'_, PyString>, class_: Option<PyComponentClass>) -> Self {
        let py = name.py();

        let name = name.clone().unbind();
        let ports = PyDict::new_bound(py).unbind();
        let references = PyDict::new_bound(py).unbind();
        let connections = PyList::empty_bound(py).unbind();

        Self {
            name,
            ports,
            references,
            connections,
            class_,
        }
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let name = PyString::new_bound(py, self.name.bind(py).to_str()?);
        let mut component = PyComponent::new(&name, self.class_);

        let ports = self.ports.bind(py);
        iter_dict_items!(for (name: PyString, port: PyPort) in ports => {
            component.add_port(name, port)?;
        });

        let references = self.references.bind(py);
        iter_dict_items!(for (alias: PyString, reference: PyComponentRef) in references => {
            let reference = reference.borrow();
            let n_instances = reference.n_instances;
            let reference = Bound::new(py, reference.component.clone())?;
            component.add_reference(&reference, Some(alias), Some(n_instances))?;
        });

        let connections = self.connections.bind(py);
        iter_list_items!(for (connection: PyConnection) in connections => {
            let connection = connection.borrow();
            let source_pins = Bound::new(py, connection.source_pins.clone())?;
            let source_component = if let Some(ref component) = connection.source_component {
                Some(component.bind(py))
            } else {
                None
            };
            let sink_pins = Bound::new(py, connection.sink_pins.clone())?;
            let sink_component = if let Some(ref component) = connection.sink_component {
                Some(component.bind(py))
            } else {
                None
            };
            component.add_connection(&source_pins, &sink_pins, source_component, sink_component)?;
        });

        Ok(component)
    }

    pub fn add_port(
        &mut self,
        name: &Bound<'_, PyString>,
        port: &Bound<'_, PyPort>,
    ) -> PyResult<Py<PyPort>> {
        let py = name.py();

        let ports = self.ports.bind(py);

        if ports.contains(name.clone())? {
            return Err(PyValueError::new_err(format!(
                r#"port "{port}" already in "{component}""#,
                port = name.to_str()?,
                component = self.name.bind(py).to_str()?,
            )));
        }

        let port = port.borrow();
        let port = Py::new(py, port.copy(py)?)?;

        ports.set_item(name, port.clone_ref(py))?;

        Ok(port)
    }

    pub fn add_ports(&mut self, ports: &Bound<'_, PyMapping>) -> PyResult<()> {
        iter_mapping_items!(for (name: PyString, port: PyPort) in ports => {
            self.add_port(name, port)?;
        });

        Ok(())
    }

    pub fn add_reference(
        &mut self,
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<'_, PyString>>,
        n_instances: Option<usize>,
    ) -> PyResult<Py<PyComponentRef>> {
        let py = component.py();

        let alias = match alias {
            Some(alias) => alias.clone(),
            None => {
                let component = component.borrow();
                let alias = component.name.bind(py);
                PyString::new_bound(py, alias.to_str()?)
            }
        };

        let references = self.references.bind(py);
        if references.deref().contains(alias.clone())? {
            return Err(PyValueError::new_err(format!(
                r#"component or alias "{alias}" already referenced in "{component}""#,
                alias = alias.to_str()?,
                component = self.name.bind(py).to_str()?
            )));
        }

        let reference = PyComponentRef::new(component, Some(&alias), n_instances);
        let reference = Bound::new(py, reference)?;

        references.deref().set_item(alias, reference.clone())?;

        Ok(reference.unbind())
    }

    pub fn add_connection(
        &mut self,
        source_pins: &Bound<'_, PyPortPins>,
        sink_pins: &Bound<'_, PyPortPins>,
        source_component: Option<&Bound<'_, PyComponentRef>>,
        sink_component: Option<&Bound<'_, PyComponentRef>>,
    ) -> PyResult<Py<PyConnection>> {
        let py = source_pins.py();

        // TODO: check for duplicate connections?

        let connections = self.connections.bind(py);
        let connection = PyConnection::new(
            source_pins.clone(),
            sink_pins.clone(),
            source_component.cloned(),
            sink_component.cloned(),
        );
        let connection = Bound::new(py, connection)?;

        connections.append(connection.clone())?;

        Ok(connection.unbind())
    }
}

#[pyclass]
pub struct PyComponentRef {
    #[pyo3(get, set)]
    pub component: Py<PyComponent>,
    #[pyo3(get, set)]
    pub alias: Option<Py<PyString>>,
    #[pyo3(get, set)]
    pub n_instances: usize,
}

#[pymethods]
impl PyComponentRef {
    #[new]
    pub fn new(
        component: &Bound<'_, PyComponent>,
        alias: Option<&Bound<PyString>>,
        n_instances: Option<usize>,
    ) -> Self {
        let component = component.clone().unbind();
        let alias = alias.map(|alias| alias.clone().unbind());
        let n_instances = n_instances.unwrap_or(1);

        Self {
            component,
            alias,
            n_instances,
        }
    }
}

#[pyclass]
pub struct PyConnection {
    #[pyo3(get, set)]
    pub source_pins: Py<PyPortPins>,
    #[pyo3(get, set)]
    pub source_component: Option<Py<PyComponentRef>>,
    #[pyo3(get, set)]
    pub sink_pins: Py<PyPortPins>,
    #[pyo3(get, set)]
    pub sink_component: Option<Py<PyComponentRef>>,
}

#[pymethods]
impl PyConnection {
    #[new]
    pub fn new(
        source_pins: Bound<'_, PyPortPins>,
        sink_pins: Bound<'_, PyPortPins>,
        source_component: Option<Bound<'_, PyComponentRef>>,
        sink_component: Option<Bound<'_, PyComponentRef>>,
    ) -> Self {
        let source_pins = source_pins.unbind();
        let source_component = source_component.map(|c| c.unbind());
        let sink_pins = sink_pins.unbind();
        let sink_component = sink_component.map(|c| c.unbind());

        Self {
            source_pins,
            source_component,
            sink_pins,
            sink_component,
        }
    }
}
