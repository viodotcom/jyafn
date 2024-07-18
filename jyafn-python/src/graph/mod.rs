mod indexed;
mod r#ref;

pub use indexed::IndexedList;
pub use r#ref::Ref;

use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyTuple};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use super::layout::Layout;
use super::{depythonize_ref_value, pythonize_ref_value, Function, ToPyErr};

thread_local! {
    pub static CONTEXT: RefCell<Vec<Graph>> =
        RefCell::new(vec![Graph::new(Some("main".to_string()))]);
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
    f(&mut lock)
}

pub fn insert_in_current<O: rust::Op>(op: O, args: Vec<rust::Ref>) -> PyResult<Ref> {
    try_with_current(|g| Ok(Ref(g.insert(op, args).map_err(ToPyErr)?)))
}

#[pyclass(module = "jyafn")]
#[derive(Clone)]
pub struct Graph(pub(crate) Arc<Mutex<rust::Graph>>);

#[pymethods]
impl Graph {
    #[new]
    #[pyo3(signature = (name=None))]
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

    #[pyo3(signature = (*args, **kwargs))]
    fn __call__(
        &self,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        if Arc::ptr_eq(&self.0, &current_graph()?.0) {
            return Err(exceptions::PyException::new_err(format!(
                "tried to call graph {} from itself. Recursion in JYAFN is disallowed.",
                self.name()
            )));
        }

        try_with_current(|g| {
            // Check if self is not the current graph.
            let graph = self.0.lock().expect("poisoned");
            let kwargs = kwargs
                .cloned()
                .unwrap_or_else(|| PyDict::new_bound(args.py()));
            let s = graph.input_layout();

            if kwargs.len() + args.len() != s.0.len() {
                return Err(exceptions::PyTypeError::new_err(format!(
                    "graph takes {} arguments but {} were given",
                    s.0.len(),
                    kwargs.len()
                )));
            }

            if !args.is_empty() {
                for (item, (name, _)) in args.iter().zip(&s.0) {
                    kwargs.set_item(name, item)?;
                }
            }

            let kwargs_ref = depythonize_ref_value(g, &kwargs)?;
            let graph_id = g.insert_subgraph(graph.clone());
            let output = g.call_graph(graph_id, kwargs_ref).map_err(ToPyErr)?;

            pythonize_ref_value(args.py(), output)
        })
    }

    fn get_size(&self) -> usize {
        get_size::GetSize::get_size(&*self.0.lock().expect("poisoned"))
    }

    pub fn __getstate__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.dump(py)
    }

    pub fn dump<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let mut bytes = Vec::<u8>::new();
        self.0
            .lock()
            .expect("poisoned")
            .dump(std::io::Cursor::new(&mut bytes))
            .map_err(ToPyErr)?;
        Ok(PyBytes::new_bound(py, &bytes))
    }

    pub fn write(&self, path: &str) -> PyResult<()> {
        let file = std::fs::File::create(path)?;
        self.0
            .lock()
            .expect("poisoned")
            .dump(file)
            .map_err(ToPyErr)?;
        Ok(())
    }

    #[staticmethod]
    pub fn load(bytes: &Bound<'_, PyBytes>) -> PyResult<Self> {
        Ok(Graph(Arc::new(Mutex::new(
            rust::Graph::load(std::io::Cursor::new(bytes.as_bytes())).map_err(ToPyErr)?,
        ))))
    }

    pub fn __setstate__(&self, bytes: &Bound<'_, PyBytes>) -> PyResult<()> {
        *self.0.lock().expect("poisoned") =
            rust::Graph::load(std::io::Cursor::new(bytes.as_bytes())).map_err(ToPyErr)?;
        Ok(())
    }

    pub fn to_json(&self) -> String {
        self.0.lock().expect("poisoned").to_json()
    }

    #[getter]
    pub fn name(&self) -> String {
        self.0.lock().expect("poisoned").name().to_string()
    }

    #[getter]
    pub fn input_layout(&self) -> Layout {
        Layout(rust::layout::Layout::Struct(
            self.0.lock().expect("poisoned").input_layout().clone(),
        ))
    }

    #[getter]
    pub fn output_layout(&self) -> Layout {
        Layout(self.0.lock().expect("poisoned").output_layout().clone())
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
        Ok(Function {
            inner: Some(
                self.0
                    .lock()
                    .expect("poisoned")
                    .compile()
                    .map_err(ToPyErr)?,
            ),
            original: None,
        })
    }
}
