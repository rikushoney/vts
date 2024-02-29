use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_arch::{ComponentClass, PortClass, PortKind};

macro_rules! wrap_enum {
    ($py_name:ident => $name:ident : $($variant:ident = $py_variant:ident $(,)*)+) => {
        #[pyclass]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum $py_name {
            $(
                $py_variant,
            )*
        }

        impl From<$py_name> for $name {
            fn from(py_kind: $py_name) -> Self {
                match py_kind {
                    $(
                        $py_name::$py_variant => { $name::$variant }
                    )*
                }
            }
        }

        impl From<$name> for $py_name {
            fn from(kind: $name) -> Self {
                match kind {
                    $(
                        $name::$variant => { $py_name::$py_variant }
                    )*
                }
            }
        }
    }
}

#[pyclass]
pub struct PyComponent {
    #[pyo3(get)]
    pub name: Py<PyString>,
    #[pyo3(get)]
    pub ports: Py<PyDict>,
    #[pyo3(get)]
    pub children: Py<PyDict>,
    #[pyo3(get)]
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
        self.ports.as_ref(py).set_item(name, port.clone_ref(py))
    }

    pub fn add_ports(&mut self, py: Python<'_>, ports: &PyMapping) -> PyResult<()> {
        self.ports.as_ref(py).update(ports)
    }
}

wrap_enum!(PyComponentClass => ComponentClass:
    Lut = LUT,
    Latch = LATCH,
);

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPort {
    #[pyo3(get)]
    pub name: Py<PyString>,
    #[pyo3(get)]
    pub kind: PyPortKind,
    #[pyo3(get)]
    pub n_pins: usize,
    #[pyo3(get)]
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
}

wrap_enum!(PyPortClass => PortClass:
    Clock = CLOCK,
    LutIn = LUT_IN,
    LutOut = LUT_OUT,
    LatchIn = LATCH_IN,
    LatchOut = LATCH_OUT,
);

wrap_enum!(PyPortKind => PortKind:
    Input = INPUT,
    Output = OUTPUT,
);

#[pymodule]
#[pyo3(name = "_vts_api_rs")]
fn vts_api_rs(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyComponent>()?;
    m.add_class::<PyComponentClass>()?;
    m.add_class::<PyPort>()?;
    m.add_class::<PyPortClass>()?;
    m.add_class::<PyPortKind>()?;
    Ok(())
}
