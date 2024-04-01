use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMapping, PyString};
use vts_core::arch::Module;

use crate::arch::{
    convert::{Converter, ModuleConverter, PyModuleConvertError, PyModuleConverter},
    PyComponent,
};

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
    pub fn new(name: &Bound<PyString>) -> Self {
        let py = name.py();
        let name = name.clone().unbind();
        let components = PyDict::new_bound(py).unbind();

        Self { name, components }
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let name = PyString::new_bound(py, self.name.bind(py).to_str()?);
        let mut module = PyModule_::new(&name);

        let components = self.components.bind(py);
        for (name, component) in components.iter() {
            let name = name.downcast::<PyString>()?;
            let component = component.downcast::<PyComponent>()?;
            module.add_component(name, component)?;
        }

        Ok(module)
    }

    pub fn add_component(
        &mut self,
        name: &Bound<'_, PyString>,
        component: &Bound<'_, PyComponent>,
    ) -> PyResult<Py<PyComponent>> {
        let py = name.py();
        let components = self.components.bind(py);

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

    pub fn add_components(&mut self, components: &Bound<'_, PyMapping>) -> PyResult<()> {
        iter_mapping_items!(for (name: PyString, component: PyComponent) in components => {
            self.add_component(name, component)?;
        });

        Ok(())
    }
}

#[pyfunction]
pub fn json_loads(input: Bound<'_, PyString>) -> PyResult<Py<PyModule_>> {
    let py = input.py();

    let input = input.downcast::<PyString>()?;
    let module: Module = map_serde_py_err!(serde_json::from_str(input.to_str()?))?;
    let converter = ModuleConverter(py, module);

    converter.convert()
}

#[pyfunction]
pub fn json_dumps(
    py: Python<'_>,
    module: &Bound<'_, PyModule_>,
    pretty: bool,
) -> PyResult<Py<PyString>> {
    let converter = PyModuleConverter(module.clone());
    let module = converter.convert().map_err(|err| match err {
        PyModuleConvertError::Python(err) => err,
        _ => PyValueError::new_err(format!("{err}")),
    })?;

    let json = if pretty {
        map_serde_py_err!(serde_json::to_string_pretty(&module))?
    } else {
        map_serde_py_err!(serde_json::to_string(&module))?
    };

    let json = PyString::new_bound(py, json.as_str());
    Ok(json.into_py(py))
}
