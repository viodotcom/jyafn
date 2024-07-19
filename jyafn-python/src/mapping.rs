use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::layout::Obj;
use super::{depythonize_ref_value, graph, pythonize_ref_value, Layout, ToPyErr};

#[pyclass(module = "jyafn")]
pub struct LazyMapping {
    is_consumed: bool,
    name: String,
    key_layout: rust::layout::Layout,
    value_layout: rust::layout::Layout,
    obj: PyObject,
}

impl LazyMapping {
    fn init(&mut self, py: Python, g: &mut rust::Graph) -> PyResult<()> {
        if !g.mappings().contains_key(&self.name) {
            if let Ok(dict) = self.obj.downcast_bound::<PyDict>(py) {
                g.insert_mapping(
                    self.name.clone(),
                    self.key_layout.clone(),
                    self.value_layout.clone(),
                    rust::mapping::HashMapStorage,
                    dict.iter().map(|(k, v)| (Obj(k), Obj(v))).map(Ok),
                )?;
            } else {
                if self.is_consumed {
                    return Err(exceptions::PyException::new_err(
                        "LazyMapping is already consumed. You initialized this LazyMapping with an \
                        iterator and this iterator has been consumed.",
                    ));
                }

                // Fallible tuple iterator:
                let iter = self.obj.bind(py).iter()?.map(|item| {
                    item.and_then(|i| {
                        i.extract::<(Bound<PyAny>, Bound<PyAny>)>()
                            .map(|(k, v)| (Obj(k), Obj(v)))
                    })
                });

                g.insert_mapping(
                    self.name.clone(),
                    self.key_layout.clone(),
                    self.value_layout.clone(),
                    rust::mapping::HashMapStorage,
                    iter,
                )?;
            }
        }

        self.is_consumed = true;

        Ok(())
    }
}

#[pymethods]
impl LazyMapping {
    #[new]
    fn new(name: String, key_layout: Layout, value_layout: Layout, obj: PyObject) -> Self {
        Self {
            is_consumed: false,
            name,
            key_layout: key_layout.0,
            value_layout: value_layout.0,
            obj,
        }
    }

    fn __getitem__(&mut self, key: &Bound<PyAny>) -> PyResult<PyObject> {
        graph::try_with_current(|g| {
            let ref_value = depythonize_ref_value(g, key)?;
            self.init(key.py(), g)?;
            let value = g.call_mapping(&self.name, ref_value).map_err(ToPyErr)?;
            pythonize_ref_value(key.py(), value)
        })
    }

    // /// See issue https://github.com/PyO3/pyo3/issues/4051
    // fn __contains__(&self, key: &Bound<PyAny>) -> PyResult<PyObject> {
    //     todo()
    // }

    #[pyo3(signature = (key, default=None))]
    fn get(&mut self, key: &Bound<PyAny>, default: Option<&Bound<PyAny>>) -> PyResult<PyObject> {
        if let Some(default) = default {
            graph::try_with_current(|g| {
                let ref_value = depythonize_ref_value(g, key)?;
                let default_value = depythonize_ref_value(g, default)?;
                self.init(key.py(), g)?;
                let value = g
                    .call_mapping_default(&self.name, ref_value, default_value)
                    .map_err(ToPyErr)?;
                pythonize_ref_value(key.py(), value)
            })
        } else {
            self.__getitem__(key)
        }
    }
}
