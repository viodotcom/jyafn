use super::layout::{Obj, PyDecoder};
use super::{Function, Layout, ToPyErr};
use pyo3::prelude::*;
use pyo3::types::PyList;

pub fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Dataset>()?;

    Ok(())
}

#[pyclass]
struct Dataset(rust::Dataset);

#[pymethods]
impl Dataset {
    #[staticmethod]
    fn build(layout: &Layout, data: Bound<'_, PyAny>) -> PyResult<Dataset> {
        Ok(Dataset(rust::Dataset::try_build(
            layout.0.clone(),
            |e| ToPyErr(e).into(),
            data.iter()?.map(|item| Ok::<_, PyErr>(Obj(item?))),
        )?))
    }

    fn map(&self, func: &Function) -> PyResult<Dataset> {
        Ok(Dataset(self.0.map(&func.0).map_err(ToPyErr)?))
    }

    fn decode(&self, py: Python) -> Py<PyList> {
        PyList::new_bound(py, self.0.decode_with_decoder(PyDecoder(py))).unbind()
    }
}
