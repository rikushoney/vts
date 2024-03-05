use pyo3::prelude::*;

macro_rules! wrap_enum {
    ($py_name:ident => $name:ident : $($py_variant:ident = $variant:ident $(| $($alias:literal)+)? $(,)*)+) => {
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

mod arch;

#[pymodule]
#[pyo3(name = "_vts_api_rs")]
fn vts_api_rs(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<arch::PyModule_>()?;
    m.add_class::<arch::PyComponent>()?;
    m.add_class::<arch::PyComponentClass>()?;
    m.add_class::<arch::PyPort>()?;
    m.add_class::<arch::PyPortClass>()?;
    m.add_class::<arch::PyPortKind>()?;
    Ok(())
}
