use pyo3::prelude::*;

macro_rules! wrap_enum {
    ($py_name:ident (
        name = $py_rename:literal,
        help = $py_alias:expr
    ) => $name:ident :
        $(
            $py_variant:ident = $variant:ident (alias = $alias:pat) $(,)*
        )+
    ) => {
        #[pyclass(name = $py_rename)]
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum $py_name {
            $(
                $py_variant,
            )*
        }

        #[pymethods]
        impl $py_name {
            fn __str__(&self) -> &'static str {
                match self {
                    $(
                        Self::$py_variant => stringify!($py_variant),
                    )+
                }
            }
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

        impl std::str::FromStr for $py_name {
            type Err = String;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                let lower = s.to_lowercase();
                match lower.as_str() {
                    $(
                        $alias => Ok($py_name::$py_variant),
                    )+
                    _ => {
                        Err(s.to_string())
                    }
                }
            }
        }
    }
}

mod arch;

macro_rules! register_modules {
    ($py:ident + $super:ident { $($module:ident :: $register:ident),* $(,)? }) => {
        let sys_modules = $py.import_bound("sys")?.getattr("modules")?;

        $(
            let submodule = $module::$register($super)?;
            let submodule_name = concat!("vts._vts.", stringify!($module));
            sys_modules.set_item(submodule_name, submodule.clone())?;
            submodule.setattr("__name__", submodule_name)?;
        )*
    };
}

#[pymodule]
fn _vts(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_modules!(py + module { arch::register_arch });
    Ok(())
}
