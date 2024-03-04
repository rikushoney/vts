use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_arch::{ComponentClass, PortClass, PortKind};

#[pyclass]
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
    ) -> PyResult<()> {
        let components = self.components.as_ref(py);
        if components.contains(name)? {
            return Err(PyValueError::new_err(format!(
                r#"component with name "{name}" already in "{}""#,
                self.name
            )));
        }
        components.set_item(name, component.clone_ref(py))
    }

    pub fn add_components(&mut self, py: Python<'_>, components: &PyMapping) -> PyResult<()> {
        for (name, component) in components
            .items()?
            .iter()?
            .map(|x| x.and_then(PyAny::extract::<(Py<PyString>, Py<PyComponent>)>))
            .collect::<PyResult<Vec<(Py<PyString>, Py<PyComponent>)>>>()?
            .iter()
        {
            self.add_component(py, name.as_ref(py).to_str()?, component.clone_ref(py))?;
        }
        Ok(())
    }
}

#[pyclass]
pub struct PyComponent {
    #[pyo3(get, set)]
    pub name: Py<PyString>,
    #[pyo3(get, set)]
    pub ports: Py<PyDict>,
    #[pyo3(get, set)]
    pub children: Py<PyDict>,
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
            children: PyDict::new(py).into(),
            class_,
        })
    }

    pub fn add_port(&mut self, py: Python<'_>, name: &str, port: Py<PyPort>) -> PyResult<()> {
        let ports = self.ports.as_ref(py);
        if ports.contains(name)? {
            return Err(PyValueError::new_err(format!(
                r#"port with name "{name}" already in "{}""#,
                self.name
            )));
        }
        ports.set_item(name, port.clone_ref(py))
    }

    pub fn add_ports(&mut self, py: Python<'_>, ports: &PyMapping) -> PyResult<()> {
        for (name, port) in ports
            .items()?
            .iter()?
            .map(|x| x.and_then(PyAny::extract::<(Py<PyString>, Py<PyPort>)>))
            .collect::<PyResult<Vec<(Py<PyString>, Py<PyPort>)>>>()?
            .iter()
        {
            self.add_port(py, name.as_ref(py).to_str()?, port.clone_ref(py))?;
        }
        Ok(())
    }
}

wrap_enum!(PyComponentClass => ComponentClass:
    LUT = Lut,
    LATCH = Latch,
);

#[pyfunction]
pub fn _component_class_from_str(class: &str) -> PyResult<PyComponentClass> {
    match class.to_lowercase().as_str() {
        "lut" => Ok(PyComponentClass::LUT),
        "latch" | "ff" => Ok(PyComponentClass::LATCH),
        _ => Err(PyValueError::new_err(format!(
            r#"unknown component class "{class}""#
        ))),
    }
}

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
    ) -> Self {
        let n_pins = n_pins.unwrap_or(1);
        let name = PyString::new(py, name).into_py(py);
        Self {
            name,
            kind,
            n_pins,
            class_,
        }
    }

    pub fn copy(&self) -> Self {
        self.clone()
    }
}

wrap_enum!(PyPortClass => PortClass:
    CLOCK = Clock,
    LUT_IN = LutIn,
    LUT_OUT = LutOut,
    LATCH_IN = LatchIn,
    LATCH_OUT = LatchOut,
);

#[pyfunction]
pub fn _port_class_from_str(class: &str) -> PyResult<PyPortClass> {
    match class.to_lowercase().as_str() {
        "clock" | "clk" => Ok(PyPortClass::CLOCK),
        "lut_in" => Ok(PyPortClass::LUT_IN),
        "lut_out" => Ok(PyPortClass::LUT_OUT),
        "latch_in" | "ff_in" => Ok(PyPortClass::LATCH_IN),
        "latch_out" | "ff_out" => Ok(PyPortClass::LATCH_OUT),
        _ => Err(PyValueError::new_err(format!(
            r#"unknown port class "{class}""#
        ))),
    }
}

wrap_enum!(PyPortKind => PortKind:
    INPUT = Input,
    OUTPUT = Output,
);

#[pyfunction]
pub fn _port_kind_from_str(kind: &str) -> PyResult<PyPortKind> {
    match kind.to_lowercase().as_str() {
        "input" | "in" | "i" => Ok(PyPortKind::INPUT),
        "output" | "out" | "o" => Ok(PyPortKind::OUTPUT),
        _ => Err(PyValueError::new_err(format!(
            r#"unknown port kind "{kind}""#
        ))),
    }
}
