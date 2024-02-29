use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use vts_arch::{Component, ComponentClass, Port, PortClass, PortKind};

macro_rules! wrap_enum {
    ($py_name:ident => $name:ident : $($variant:ident = $py_variant:ident $(,)*)+) => {
        #[pyclass]
        #[allow(non_camel_case_types)]
        #[derive(Clone, Debug, PartialEq)]
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
    pub ports: Py<PyDict>,
    pub children: Py<PyDict>,
    pub class: Option<PyComponentClass>,
}

#[pymethods]
impl PyComponent {
    #[new]
    pub fn new(py: Python<'_>, class: Option<PyComponentClass>) -> Self {
        Self {
            ports: PyDict::new(py).into(),
            children: PyDict::new(py).into(),
            class,
        }
    }

    pub fn add_port(&mut self, py: Python<'_>, name: &str, port: Py<PyPort>) -> PyResult<()> {
        let ports = self.ports.try_borrow_mut(py)?;
        ports.set_item(0, 3);
        Ok(())
    }
}

wrap_enum!(PyComponentClass => ComponentClass:
    Lut = LUT,
    Latch = LATCH,
);

#[pyclass]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPort {
    pub kind: PyPortKind,
    pub n_pins: usize,
    pub class: Option<PyPortClass>,
}

#[pymethods]
impl PyPort {
    #[new]
    pub fn new() -> Self {
        todo!()
        // Self {}
    }
}

wrap_enum!(PyPortKind => PortKind:
    Input = INPUT,
    Output = OUTPUT,
);

wrap_enum!(PyPortClass => PortClass:
    Clock = CLOCK,
    LutIn = LUT_IN,
    LutOut = LUT_OUT,
    LatchIn = LATCH_IN,
    LatchOut = LATCH_OUT,
);

#[pymodule]
fn vts_api_rs(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyComponent>()?;
    m.add_class::<PyComponentClass>()?;
    m.add_class::<PyPort>()?;
    m.add_class::<PyPortKind>()?;
    m.add_class::<PyPortClass>()?;
    Ok(())
}
