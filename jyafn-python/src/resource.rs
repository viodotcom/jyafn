use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use super::{depythonize_ref_value, graph, pythonize_ref_value, ToPyErr};

#[pyclass(module = "jyafn")]
pub struct ResourceType(Box<dyn rust::resource::ResourceType>);

#[pymethods]
impl ResourceType {
    #[staticmethod]
    pub(crate) fn from_json(json: &str) -> PyResult<Self> {
        let value: Box<dyn rust::resource::ResourceType> =
            serde_json::from_str(json).map_err(|e| ToPyErr(e.to_string().into()))?;
        Ok(Self(value))
    }

    fn load(&self, name: String, bytes: &[u8]) -> PyResult<LazyResource> {
        let resource = self
            .0
            .from_bytes(bytes)
            .map_err(|e| ToPyErr(e.to_string().into()))?;

        Ok(LazyResource {
            resource: Arc::new(Mutex::new(Some(resource))),
            name,
        })
    }
}

#[pyclass(module = "jyafn")]
pub struct LazyResource {
    resource: Arc<Mutex<Option<Pin<Box<dyn rust::resource::Resource>>>>>,
    name: String,
}

#[pymethods]
impl LazyResource {
    fn __getattr__(&self, method_name: String) -> LazyResourceCall {
        LazyResourceCall {
            resource: self.resource.clone(),
            name: self.name.clone(),
            method_name,
        }
    }
}

#[pyclass(module = "jyafn")]
pub struct LazyResourceCall {
    resource: Arc<Mutex<Option<Pin<Box<dyn rust::resource::Resource>>>>>,
    name: String,
    method_name: String,
}

impl LazyResourceCall {
    fn init(&self, g: &mut rust::Graph) {
        let Some(resource) = self.resource.lock().expect("poisoned").take() else {
            return;
        };

        g.insert_resource_boxed(self.name.clone(), resource);
    }
}

#[pymethods]
impl LazyResourceCall {
    #[pyo3(signature = (**kwargs))]
    fn __call__(&self, py: Python, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
        graph::try_with_current(|g| {
            self.init(g);

            let input = if let Some(kwargs) = kwargs {
                depythonize_ref_value(g, kwargs)?
            } else {
                rust::layout::RefValue::Struct(HashMap::new())
            };
            let output = g
                .call_resource(&self.name, &self.method_name, input)
                .map_err(|e| ToPyErr(e.to_string().into()))?;

            pythonize_ref_value(py, output)
        })
    }
}
