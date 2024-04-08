mod r#ref;

pub use r#ref::Ref;

use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyString};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use super::{Function, ToPyErr};

thread_local! {
    pub static CONTEXT: RefCell<Vec<Graph>> = RefCell::new(vec![Graph::new(Some("main".to_string()))]);
}

#[pyfunction]
pub fn current_graph() -> PyResult<Graph> {
    CONTEXT.with_borrow(|context| {
        context
            .last()
            .cloned()
            .ok_or_else(|| exceptions::PyException::new_err("no current graph found"))
    })
}

pub fn try_with_current<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce(&mut rust::Graph) -> PyResult<T>,
{
    let current = current_graph()?;
    let mut lock = current.0.lock().expect("poisoned");
    f(&mut *lock)
}

pub fn with_current<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce(&mut rust::Graph) -> T,
{
    try_with_current(|g| Ok(f(g)))
}

pub fn insert_in_current<O: rust::Op>(op: O, args: Vec<rust::Ref>) -> PyResult<Ref> {
    try_with_current(|g| Ok(Ref(g.insert(op, args).map_err(ToPyErr)?)))
}

#[pyclass]
#[derive(Clone)]
pub struct Graph(pub(crate) Arc<Mutex<rust::Graph>>);

#[pymethods]
impl Graph {
    #[new]
    pub fn new(name: Option<String>) -> Graph {
        if let Some(name) = name {
            Graph(Arc::new(Mutex::new(rust::Graph::new_with_name(name))))
        } else {
            Graph(Arc::new(Mutex::new(rust::Graph::new())))
        }
    }

    fn __repr__(&self) -> String {
        format!("Graph(name={:?})", self.0.lock().expect("poisoned").name())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        CONTEXT.with_borrow_mut(|context| {
            context.push(slf.clone());
        });
        slf
    }

    #[allow(unused_variables)]
    fn __exit__(&self, exc_type: PyObject, exc_val: PyObject, exc_tb: PyObject) {
        CONTEXT.with_borrow_mut(|context| {
            context.pop();
        });
    }

    pub fn dump<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let data = self.0.lock().expect("poisoned").dump();
        let leaked = Box::leak(data.into_boxed_slice());
        // Safety: leaking the box from rust and giving it to Python. Therefore, no
        // double free.
        unsafe { PyBytes::bound_from_ptr(py, leaked.as_ptr(), leaked.len()) }
    }

    #[staticmethod]
    pub fn load(bytes: &Bound<'_, PyBytes>) -> PyResult<Self> {
        Ok(Graph(Arc::new(Mutex::new(
            rust::Graph::load(bytes.as_bytes()).map_err(ToPyErr)?,
        ))))
    }

    pub fn to_json(&self) -> String {
        self.0.lock().expect("poisoned").to_json()
    }

    #[staticmethod]
    pub fn from_json(json: &Bound<'_, PyString>) -> PyResult<Self> {
        Ok(Graph(Arc::new(Mutex::new(
            rust::Graph::from_json(json.to_str()?).map_err(ToPyErr)?,
        ))))
    }

    #[getter]
    pub fn metadata<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new_bound(py);

        for (key, value) in self.0.lock().expect("poisoned").metadata() {
            dict.set_item(key, value)?
        }

        Ok(dict)
    }

    pub fn set_metadata(&self, key: String, value: String) {
        self.0
            .lock()
            .expect("poisoned")
            .metadata_mut()
            .insert(key, value);
    }

    fn render(&self) -> String {
        self.0.lock().expect("poisoned").render().to_string()
    }

    fn render_assembly(&self) -> PyResult<String> {
        Ok(self
            .0
            .lock()
            .expect("poisoned")
            .render_assembly()
            .map_err(ToPyErr)?)
    }

    fn compile(&self) -> PyResult<Function> {
        Ok(Function{
            inner: self.0
                .lock()
                .expect("poisoned")
                .compile()
                .map_err(ToPyErr)?,
            original: None
        })
    }
}
