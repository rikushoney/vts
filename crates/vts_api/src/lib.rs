use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use vts_arch::{Component, ComponentClass, Port, PortClass, PortKind};

macro_rules! wrap_enum {
    ($py_name:ident => $name:ident : $variant0:ident, $($variant:ident),*$(,)*) => {
        #[pyclass]
        pub enum $py_name {
            $variant0,
            $(
                $variant,
            )*
        }

        impl From<$py_name> for $name {
            fn from(py_kind: $py_name) -> Self {
                match py_kind {
                    $py_name::$variant0 => { $name::$variant0 }
                    $(
                        $py_name::$variant => { $name::$variant }
                    )*
                }
            }
        }

        impl From<$name> for $py_name {
            fn from(kind: $name) -> Self {
                match kind {
                    $name::$variant0 => { $py_name::$variant0 }
                    $(
                        $name::$variant => { $py_name::$variant }
                    )*
                }
            }
        }
    }
}

#[pyclass]
#[repr(transparent)]
pub struct PyComponent {
    component: Arc<Component>,
}

#[pymethods]
impl PyComponent {
    #[new]
    pub fn new() -> Self {
        Self {
            component: Arc::new(Component::default()),
        }
    }

    pub fn add_port(&mut self, _name: &str, _port: PyPort) -> PyResult<()> {
        Ok(())
    }

    pub fn ports_to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (name, port) in self.component.ports.iter() {
            let port = PyPort { port: port.clone() };
            dict.set_item(name.to_string(), port.into_py(py))?;
        }
        Ok(dict.into())
    }
}

wrap_enum!(PyComponentClass => ComponentClass:
    Lut,
    Latch,
);

#[pyclass]
#[repr(transparent)]
pub struct PyPort {
    #[allow(dead_code)]
    port: Arc<Port>,
}

impl Clone for PyPort {
    fn clone(&self) -> Self {
        Self {
            port: Arc::new(Port::new(PortKind::Input, 0, None)),
        }
    }
}

#[pymethods]
impl PyPort {
    #[new]
    pub fn new() -> Self {
        Self {
            port: Arc::new(Port::new(PortKind::Input, 0, None)),
        }
    }
}

wrap_enum!(PyPortKind => PortKind:
    Input,
    Output,
);

wrap_enum!(PyPortClass => PortClass:
    Clock,
    LutIn,
    LutOut,
    LatchIn,
    LatchOut,
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
