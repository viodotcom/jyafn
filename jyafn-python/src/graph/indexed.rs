use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::PyList;

use super::{depythonize_ref_value, pythonize_ref_value, try_with_current, Ref, ToPyErr};

#[pyclass(module = "jyafn")]
#[derive(Clone)]
pub struct IndexedList {
    layout: rust::layout::Layout,
    lists: Vec<rust::IndexedList>,
}

#[pymethods]
impl IndexedList {
    #[new]
    fn new(options: &Bound<PyList>) -> PyResult<IndexedList> {
        try_with_current(|g| {
            let depythonized = options
                .iter()
                .map(|item| depythonize_ref_value(g, &item))
                .collect::<Result<Vec<_>, _>>()?;
            let Some(first) = depythonized.first() else {
                return Ok(IndexedList {
                    layout: rust::layout::Layout::Scalar,
                    lists: vec![],
                });
            };

            let layout = first.putative_layout();
            if let Some(different) = depythonized
                .iter()
                .find(|v| v.putative_layout() != layout)
            {
                return Err(exceptions::PyTypeError::new_err(format!(
                    "not all elements in list have the same layout. Expected {layout} and found \
                    {different}"
                )));
            }

            let vecs = depythonized
                .iter()
                .map(|v| {
                    v.output_vec(&layout)
                        .expect("can build vec from layout here")
                })
                .collect::<Vec<_>>();
            let element_length = vecs[0].len();
            let mut iters: Vec<_> = vecs.into_iter().map(|v| v.into_iter()).collect();
            let lists = (0..element_length)
                .map(|_| {
                    g.indexed_list(
                        iters
                            .iter_mut()
                            .map(|n| n.next().expect("has next"))
                            .collect(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(ToPyErr)?;

            Ok(IndexedList { layout, lists })
        })
    }

    fn __getitem__(&self, py: Python, idx: Ref) -> PyResult<Py<PyAny>> {
        let indexed = try_with_current(|g| {
            let indexed = self
                .lists
                .iter()
                .map(|list| list.get(g, idx.0))
                .collect::<Result<Vec<_>, _>>()
                .map_err(ToPyErr)?;

            Ok(self
                .layout
                .build_ref_value(indexed)
                .expect("can build ref-value from layout here"))
        })?;

        pythonize_ref_value(py, indexed)
    }
}
