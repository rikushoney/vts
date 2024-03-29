pub mod de;
pub mod ser;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use serde::de::DeserializeSeed;

use crate::arch::{iter_mapping_items, PyComponent};

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
    pub fn new(py: Python<'_>, name: &Bound<PyString>) -> Self {
        let name = name.clone().unbind();
        let components = PyDict::new_bound(py).unbind();

        Self { name, components }
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let name = PyString::new_bound(py, self.name.bind(py).to_str()?);
        let mut module = PyModule_::new(py, &name);

        let components = self.components.bind(py);
        for (name, component) in components.iter() {
            let name = name.extract::<&str>()?;
            let component = component.downcast::<PyComponent>()?;
            module.add_component(py, name, component)?;
        }

        Ok(module)
    }

    pub fn add_component(
        &mut self,
        py: Python<'_>,
        name: &str,
        component: &Bound<'_, PyComponent>,
    ) -> PyResult<Py<PyComponent>> {
        let components = self.components.bind(py);
        let name = PyString::new_bound(py, name);

        if components.contains(name.clone())? {
            let component_name = name.to_str()?;
            let module_name = self.name.to_str(py)?;
            return Err(PyValueError::new_err(format!(
                r#"component with name "{component_name}" already in "{module_name}""#
            )));
        }

        let component = component.borrow();
        let component = Py::new(py, component.copy(py)?)?;

        components.set_item(name, component.clone_ref(py))?;

        Ok(component)
    }

    pub fn add_components(
        &mut self,
        py: Python<'_>,
        components: &Bound<'_, PyMapping>,
    ) -> PyResult<()> {
        iter_mapping_items!(for (name: &str [extract], component: PyComponent [downcast]) in components => {
            self.add_component(py, name, component)?;
        });

        Ok(())
    }
}

macro_rules! map_serde_py_err {
    ($expr:expr) => {
        ($expr).map_err(|err| PyValueError::new_err(format!("{err}")))
    };
}

#[pyfunction]
pub fn json_loads(py: Python<'_>, input: &str) -> PyResult<Py<PyModule_>> {
    let json: serde_json::Value = map_serde_py_err!(serde_json::from_str(input))?;
    let module_deserializer = de::ModuleDeserializer::new(py);
    let module: Bound<'_, PyModule_> = map_serde_py_err!(module_deserializer.deserialize(json))?;

    Ok(module.unbind())
}

#[pyfunction]
pub fn json_dumps(
    py: Python<'_>,
    module: &Bound<'_, PyModule_>,
    pretty: bool,
) -> PyResult<Py<PyString>> {
    let module_serializer = ser::PyModuleSerializer::new(module.clone());
    let json = if pretty {
        map_serde_py_err!(serde_json::to_string_pretty(&module_serializer))?
    } else {
        map_serde_py_err!(serde_json::to_string(&module_serializer))?
    };
    let json = PyString::new_bound(py, json.as_str());
    Ok(json.into_py(py))
}
