use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyTuple};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{Graph, Layout, ToPyErr};

#[pyclass]
pub struct Function {
    pub(crate) inner: rust::Function,
    /// The original python function that created this function
    pub(crate) original: Option<PyObject>,
}

#[pymethods]
impl Function {
    fn __repr__(&self) -> String {
        format!(
            "<jyafn {} {} -> {} at {:#x}>",
            self.inner.graph().name(),
            self.inner.input_layout(),
            self.inner.output_layout(),
            self.fn_ptr(),
        )
    }

    fn __str__(&self) -> String {
        format!(
            "<jyafn {} {} -> {} at {:#x}>",
            self.inner.graph().name(),
            self.inner.input_layout(),
            self.inner.output_layout(),
            self.fn_ptr(),
        )
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.graph().name().to_string()
    }

    #[getter]
    fn input_size(&self) -> usize {
        self.inner.input_size()
    }

    #[getter]
    fn output_size(&self) -> usize {
        self.inner.output_size()
    }

    #[getter]
    fn input_layout(&self) -> Layout {
        Layout(self.inner.input_layout().clone())
    }

    #[getter]
    fn output_layout(&self) -> Layout {
        Layout(self.inner.output_layout().clone())
    }

    #[getter]
    fn fn_ptr(&self) -> usize {
        self.inner.fn_ptr() as *const () as usize
    }

    #[getter]
    fn get_original(&self) -> Option<&PyObject> {
        self.original.as_ref()
    }

    #[setter]
    fn set_original(&mut self, original: PyObject) {
        self.original = Some(original);
    }

    fn get_size(&self) -> usize {
        get_size::GetSize::get_size(&self.inner)
    }

    #[staticmethod]
    pub fn load(bytes: &[u8]) -> PyResult<Function> {
        Ok(Function {
            inner: rust::Function::load(std::io::Cursor::new(bytes)).map_err(ToPyErr)?,
            original: None,
        })
    }

    // pub fn dump<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
    //     let mut bytes = Vec::<u8>::new();
    //     self.inner
    //         .graph()
    //         .dump(std::io::Cursor::new(&mut bytes))
    //         .map_err(ToPyErr)?;
    //     let leaked = Box::leak(bytes.into_boxed_slice());
    //     // Safety: leaking the box from rust and giving it to Python. Therefore, no
    //     // double free.
    //     unsafe { Ok(PyBytes::bound_from_ptr(py, leaked.as_ptr(), leaked.len())) }
    // }

    pub fn write(&self, path: &str) -> PyResult<()> {
        let file = std::fs::File::create(path)?;
        self.inner.graph().dump(file).map_err(ToPyErr)?;
        Ok(())
    }

    pub fn to_json(&self) -> String {
        self.inner.graph().to_json()
    }

    fn get_graph(&self) -> Graph {
        Graph(Arc::new(Mutex::new(self.inner.graph().clone())))
    }

    #[getter]
    fn metadata(&self) -> HashMap<String, String> {
        self.inner.graph().metadata().clone()
    }

    fn eval_raw(&self, args: &[u8]) -> PyResult<Vec<u8>> {
        Ok(self
            .inner
            .eval_raw(args)
            .map_err(ToPyErr)
            .map(|o| o.into_vec())?)
    }

    fn eval(&self, val: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let outcome = self.inner.eval_with_decoder(
            &crate::layout::Obj(val.clone()),
            crate::layout::PyDecoder(val.py()),
        );

        if let Err(rust::Error::EncodeError(inner)) = &outcome {
            if let Some(err) = inner.downcast_ref::<PyErr>() {
                return Err(err.clone_ref(val.py()));
            }
        }

        Ok(outcome.map_err(ToPyErr)?)
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn __call__(
        &self,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        let kwargs = kwargs
            .cloned()
            .unwrap_or_else(|| PyDict::new_bound(args.py()));
        let rust::layout::Layout::Struct(s) = self.inner.input_layout() else {
            panic!("Input should be a struct")
        };

        if kwargs.len() + args.len() != s.0.len() {
            return Err(exceptions::PyTypeError::new_err(format!(
                "jyafn takes {} arguments but {} were given",
                s.0.len(),
                kwargs.len()
            )));
        }

        if !args.is_empty() {
            for (item, (name, _)) in args.iter().zip(&s.0) {
                kwargs.set_item(name, item)?;
            }
        }

        self.eval(&kwargs)
    }
}
