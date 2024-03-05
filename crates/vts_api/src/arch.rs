use either::Either;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator, PyMapping, PyString};
use vts_arch::{ComponentClass, PortClass, PortKind};

fn to_component_class(class_: Either<PyComponentClass, &str>) -> PyResult<PyComponentClass> {
    class_.either(
        |class_| Ok(class_),
        |class_str| _component_class_from_str(class_str),
    )
}

fn to_port_class(class_: Either<PyPortClass, &str>) -> PyResult<PyPortClass> {
    class_.either(
        |class_| Ok(class_),
        |class_str| _port_class_from_str(class_str),
    )
}

fn to_port_kind(kind: Either<PyPortKind, &str>) -> PyResult<PyPortKind> {
    kind.either(|kind| Ok(kind), |kind_str| _port_kind_from_str(kind_str))
}

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

    #[pyo3(signature = (name=None, *, component=None, class_=None))]
    pub fn add_component(
        &mut self,
        py: Python<'_>,
        name: Option<Either<&str, Py<PyComponent>>>,
        component: Option<Py<PyComponent>>,
        class_: Option<Either<PyComponentClass, &str>>,
    ) -> PyResult<Py<PyComponent>> {
        let component = if let Some(component) = component {
            let mut component = component.as_ref(py).try_borrow()?.copy(py)?;

            if let Some(name) = name {
                match name {
                    Either::Left(name) => {
                        component.name = name.into_py(py);
                    }
                    Either::Right(component) => {
                        let type_name = component.as_ref(py).get_type().name()?;
                        return Err(PyTypeError::new_err(format!(
                            r#"expected "name" to be "str" not "{type_name}""#
                        )));
                    }
                }
            }

            if let Some(class_) = class_ {
                component.class_ = Some(to_component_class(class_)?);
            }

            component
        } else {
            match name {
                Some(Either::Left(name)) => PyComponent::new(py, name, class_)?,
                Some(Either::Right(component)) => component.as_ref(py).try_borrow()?.copy(py)?,
                None => {
                    return Err(PyValueError::new_err("component must have a name"));
                }
            }
        };

        let name = component.name.clone_ref(py);
        let component = Py::new(py, component)?;
        let components = self.components.as_ref(py);

        if components.contains(name.clone_ref(py))? {
            let component_name = name.as_ref(py).to_str()?;
            let module_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"component with name "{component_name}" already in "{module_name}""#
            )));
        }

        components.set_item(name, component.clone_ref(py))?;
        Ok(component)
    }

    pub fn add_components(
        &mut self,
        py: Python<'_>,
        components: Either<&PyMapping, &PyIterator>,
    ) -> PyResult<()> {
        match components {
            Either::Left(components) => {
                for item in components.items()?.iter()? {
                    let (name, component) = PyAny::extract::<(&str, Py<PyComponent>)>(item?)?;
                    let component = Py::new(py, component.as_ref(py).try_borrow()?.copy(py)?)?;
                    self.add_component(py, Some(Either::Left(name)), Some(component), None)?;
                }
            }
            Either::Right(components) => {
                for item in components {
                    let component = PyAny::extract::<Py<PyComponent>>(item?)?;
                    let name = component.as_ref(py).try_borrow()?.name.clone_ref(py);
                    let component = Py::new(py, component)?;
                    self.add_component(
                        py,
                        Some(Either::Left(name.as_ref(py).to_str()?)),
                        Some(component),
                        None,
                    )?;
                }
            }
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
    pub children: Py<PyDict>,
    #[pyo3(get, set)]
    pub class_: Option<PyComponentClass>,
}

#[pymethods]
impl PyComponent {
    #[new]
    pub fn new(
        py: Python<'_>,
        name: &str,
        class_: Option<Either<PyComponentClass, &str>>,
    ) -> PyResult<Self> {
        Ok(Self {
            name: PyString::new(py, name).into_py(py),
            ports: PyDict::new(py).into(),
            children: PyDict::new(py).into(),
            class_: class_.map(to_component_class).transpose()?,
        })
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        let mut component = Self::new(
            py,
            self.name.as_ref(py).to_str()?,
            self.class_.map(Either::Left),
        )?;

        for (name, port) in self.ports.as_ref(py).iter() {
            let name = PyAny::extract::<&str>(name)?;
            let port = PyAny::extract::<PyPort>(port)?.copy(py)?;
            component.add_port(
                py,
                Some(Either::Left(name)),
                Some(Py::new(py, port)?),
                None,
                None,
                None,
            )?;
        }

        for (name, component) in self.children.as_ref(py).iter() {
            let _name = PyAny::extract::<&str>(name)?;
            let _component = PyAny::extract::<PyComponent>(component)?.copy(py)?;
            // TODO: add_child(name, component)
        }

        Ok(component)
    }

    #[pyo3(signature = (name=None, *, port=None, kind=None, n_pins=None, class_=None))]
    pub fn add_port(
        &mut self,
        py: Python<'_>,
        name: Option<Either<&str, Py<PyPort>>>,
        port: Option<Py<PyPort>>,
        kind: Option<Either<PyPortKind, &str>>,
        n_pins: Option<usize>,
        class_: Option<Either<PyPortClass, &str>>,
    ) -> PyResult<Py<PyPort>> {
        let port = if let Some(port) = port {
            let mut port = port.as_ref(py).try_borrow()?.copy(py)?;

            if let Some(name) = name {
                match name {
                    Either::Left(name) => {
                        port.name = name.into_py(py);
                    }
                    Either::Right(port) => {
                        let port_type = port.as_ref(py).get_type().name()?;
                        return Err(PyTypeError::new_err(format!(
                            r#"expected "name" to be "str" not "{port_type}""#
                        )));
                    }
                }
            }

            if let Some(kind) = kind {
                port.kind = to_port_kind(kind)?;
            }

            if let Some(n_pins) = n_pins {
                port.n_pins = n_pins;
            }

            if let Some(class_) = class_ {
                port.class_ = Some(to_port_class(class_)?);
            }

            port
        } else {
            match name {
                Some(Either::Left(name)) => {
                    let kind = kind.ok_or(PyValueError::new_err("port must have a kind"))?;
                    PyPort::new(py, name, kind, n_pins, class_)?
                }
                Some(Either::Right(port)) => port.as_ref(py).try_borrow()?.copy(py)?,
                None => {
                    return Err(PyValueError::new_err("port must have a name"));
                }
            }
        };

        let name = port.name.clone_ref(py);
        let port = Py::new(py, port)?;
        let ports = self.ports.as_ref(py);

        if ports.contains(name.clone_ref(py))? {
            let port_name = name.as_ref(py).to_str()?;
            let component_name = self.name.as_ref(py).to_str()?;
            return Err(PyValueError::new_err(format!(
                r#"port with name "{port_name}" already in "{component_name}""#
            )));
        }

        ports.set_item(name, port.clone_ref(py))?;
        Ok(port)
    }

    pub fn add_ports(
        &mut self,
        py: Python<'_>,
        ports: Either<&PyMapping, &PyIterator>,
    ) -> PyResult<()> {
        match ports {
            Either::Left(ports) => {
                for item in ports.items()?.iter()? {
                    let (name, port) = PyAny::extract::<(&str, Py<PyPort>)>(item?)?;
                    let port = Py::new(py, port.as_ref(py).try_borrow()?.copy(py)?)?;
                    self.add_port(py, Some(Either::Left(name)), Some(port), None, None, None)?;
                }
            }
            Either::Right(ports) => {
                for item in ports.iter()? {
                    let port = PyAny::extract::<Py<PyPort>>(item?)?;
                    let name = port.as_ref(py).try_borrow()?.name.clone_ref(py);
                    let port = Py::new(py, port.as_ref(py).try_borrow()?.copy(py)?)?;
                    self.add_port(
                        py,
                        Some(Either::Left(name.as_ref(py).to_str()?)),
                        Some(port),
                        None,
                        None,
                        None,
                    )?;
                }
            }
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
        kind: Either<PyPortKind, &str>,
        n_pins: Option<usize>,
        class_: Option<Either<PyPortClass, &str>>,
    ) -> PyResult<Self> {
        Ok(Self {
            name: PyString::new(py, name).into_py(py),
            kind: to_port_kind(kind)?,
            n_pins: n_pins.unwrap_or(1),
            class_: class_.map(to_port_class).transpose()?,
        })
    }

    pub fn copy(&self, py: Python<'_>) -> PyResult<Self> {
        Self::new(
            py,
            self.name.as_ref(py).to_str()?,
            Either::Left(self.kind),
            Some(self.n_pins),
            self.class_.map(Either::Left),
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
