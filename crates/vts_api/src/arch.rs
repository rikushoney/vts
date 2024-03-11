use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_arch::{ComponentClass, PortClass, PortKind};

#[pyclass]
#[pyo3(name = "PyModule")]
pub struct PyModule_ {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub components: Py<PyDict>,
}

#[pymethods]
impl PyModule_ {
    #[new]
    pub fn new(py: Python<'_>, name: &str) -> Self {
        Self {
            name: PyString::new(py, name).into_py(py),
            components: PyDict::new(py).into_py(py),
        }
    }

    pub fn add_component(
        &mut self,
        py: Python<'_>,
        name: &str,
        component: Py<PyComponent>,
    ) -> PyResult<Py<PyComponent>> {
        let components = self.components.as_ref(py);
        let name = PyString::new(py, name);

        if components.contains(name)? {
            let component_name = name.to_str()?;
            let module_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"component with name "{component_name}" already in "{module_name}""#
            )));
        }

        let component = component.as_ref(py).try_borrow()?;
        let component = Py::new(py, component.copy(py)?)?;

        components.set_item(name, component.clone_ref(py))?;
        Ok(component)
    }

    pub fn add_components(&mut self, py: Python<'_>, components: &PyMapping) -> PyResult<()> {
        for item in components.items()?.iter()? {
            let (name, component) = PyAny::extract::<(&str, Py<PyComponent>)>(item?)?;
            self.add_component(py, name, component)?;
        }
        Ok(())
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyComponent {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub ports: Py<PyDict>,
    #[pyo3(get, set)]
    pub references: Py<PyDict>,
    #[pyo3(get, set)]
    pub class_: Option<PyComponentClass>,
}

#[pymethods]
impl PyComponent {
    #[new]
    pub fn new(py: Python<'_>, name: &str, class_: Option<PyComponentClass>) -> PyResult<Self> {
        Ok(Self {
            name: PyString::new(py, name).into_py(py),
            ports: PyDict::new(py).into(),
            references: PyDict::new(py).into(),
            class_,
        })
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let mut component = PyComponent::new(py, self.name.as_ref(py).to_str()?, self.class_)?;

        for item in self.ports.as_ref(py).items().iter() {
            let (name, port) = PyAny::extract::<(&str, Py<PyPort>)>(item)?;
            component.add_port(py, name, port)?;
        }

        for item in self.references.as_ref(py).items().iter() {
            let (name, reference) = PyAny::extract::<(&str, Py<PyComponent>)>(item)?;
            component.add_ref(py, name, reference)?;
        }

        Ok(component)
    }

    pub fn add_ref(
        &mut self,
        py: Python<'_>,
        name: &str,
        component: Py<PyComponent>,
    ) -> PyResult<Py<PyComponent>> {
        let references = self.references.as_ref(py);
        let name = PyString::new(py, name);

        if references.contains(name)? {
            let reference_name = name.to_str()?;
            let component_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"component with name "{reference_name}" already referenced in "{component_name}""#
            )));
        }

        let component = component.as_ref(py).try_borrow()?;
        let component = Py::new(py, component.copy(py)?)?;

        references.set_item(name, component.clone_ref(py))?;
        Ok(component)
    }

    pub fn add_port(
        &mut self,
        py: Python<'_>,
        name: &str,
        port: Py<PyPort>,
    ) -> PyResult<Py<PyPort>> {
        let ports = self.ports.as_ref(py);
        let name = PyString::new(py, name);

        if ports.contains(name)? {
            let port_name = name.to_str()?;
            let module_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"port with name "{port_name}" already in "{module_name}""#
            )));
        }

        let port = port.as_ref(py).try_borrow()?;
        let port = Py::new(py, port.copy(py)?)?;

        ports.set_item(name, port.clone_ref(py))?;
        Ok(port)
    }

    pub fn add_ports(&mut self, py: Python<'_>, ports: &PyMapping) -> PyResult<()> {
        for item in ports.items()?.iter()? {
            let (name, port) = PyAny::extract::<(&str, Py<PyPort>)>(item?)?;
            self.add_port(py, name, port)?;
        }
        Ok(())
    }
}

wrap_enum!(PyComponentClass => ComponentClass:
    LUT = Lut,
    LATCH = Latch,
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPort {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub kind: PyPortKind,
    #[pyo3(get, set)]
    pub n_pins: usize,
    #[pyo3(get, set)]
    pub class_: Option<PyPortClass>,
}

#[pymethods]
impl PyPort {
    #[new]
    pub fn new(
        py: Python<'_>,
        name: &str,
        kind: PyPortKind,
        n_pins: Option<usize>,
        class_: Option<PyPortClass>,
    ) -> PyResult<Self> {
        Ok(Self {
            name: PyString::new(py, name).into_py(py),
            kind,
            n_pins: n_pins.unwrap_or(1),
            class_,
        })
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        Self::new(
            py,
            self.name.as_ref(py).to_str()?,
            self.kind,
            Some(self.n_pins),
            self.class_,
        )
    }
}

wrap_enum!(PyPortClass => PortClass:
    CLOCK = Clock,
    LUT_IN = LutIn,
    LUT_OUT = LutOut,
    LATCH_IN = LatchIn,
    LATCH_OUT = LatchOut,
);

wrap_enum!(PyPortKind => PortKind:
    INPUT = Input,
    OUTPUT = Output,
);
