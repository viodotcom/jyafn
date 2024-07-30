use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::resource::ResourceType;

use super::ToPyErr;

#[pyclass]
pub struct Extension(Arc<rust::extension::Extension>);

#[pymethods]
impl Extension {
    #[staticmethod]
    fn list_loaded() -> HashMap<String, Vec<String>> {
        rust::extension::list()
            .into_iter()
            .map(|(name, versions)| (name, versions.into_iter().map(|v| v.to_string()).collect()))
            .collect()
    }

    #[new]
    #[pyo3(signature = (name, version_req = "*"))]
    fn new(name: &str, version_req: &str) -> PyResult<Extension> {
        let extension = rust::extension::try_get(
            name,
            &version_req.parse().map_err(|_| {
                PyValueError::new_err(format!("bad version requirement {version_req:?}"))
            })?,
        )
        .map_err(ToPyErr)?;

        Ok(Extension(extension))
    }

    #[getter]
    fn name(&self) -> String {
        self.0.name().to_string()
    }

    #[getter]
    fn version(&self) -> String {
        self.0.version().to_string()
    }

    #[getter]
    fn resources(&self) -> Vec<&str> {
        self.0.resources().collect()
    }

    fn get(&self, resource_name: &str) -> PyResult<ResourceType> {
        if !self.0.resources().any(|name| name == resource_name) {
            return Err(PyIndexError::new_err(format!(
                "Extension {:?} does not provide resource {resource_name:?}",
                self.name()
            )));
        }

        ResourceType::from_json(&format!(
            "{{\"type\":\"External\",\"extension\":{:?},\"resource\":{:?},\"version_req\":{:?}}}",
            self.name(),
            resource_name,
            self.version(),
        ))
    }
}
