extern crate jyafn as rust;

mod function;
mod graph;
mod layout;
mod mapping;
mod pfunc;

use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyNone, PyTuple};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use function::Function;
use graph::{Graph, IndexedList, Ref};
use layout::Layout;

#[pymodule]
fn jyafn(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Graph>()?;
    m.add_class::<Ref>()?;
    m.add_class::<Type>()?;
    m.add_class::<Function>()?;
    m.add_class::<IndexedList>()?;
    m.add_function(wrap_pyfunction!(read_metadata, m)?)?;
    m.add_function(wrap_pyfunction!(read_graph, m)?)?;
    m.add_function(wrap_pyfunction!(read_fn, m)?)?;
    m.add_function(wrap_pyfunction!(graph::current_graph, m)?)?;
    m.add_function(wrap_pyfunction!(r#const, m)?)?;
    m.add_function(wrap_pyfunction!(input, m)?)?;
    m.add_function(wrap_pyfunction!(ret, m)?)?;
    m.add_function(wrap_pyfunction!(assert_, m)?)?;

    m.add_class::<layout::Layout>()?;
    m.add_function(wrap_pyfunction!(putative_layout, m)?)?;

    m.add_class::<mapping::LazyMapping>()?;

    pfunc::init(m)?;

    Ok(())
}

pub struct ToPyErr(pub rust::Error);

impl From<ToPyErr> for PyErr {
    fn from(err: ToPyErr) -> PyErr {
        exceptions::PyException::new_err(err.0.to_string())
    }
}

#[pyclass]
#[derive(Clone)]
struct Type(rust::Type);

#[pymethods]
impl Type {
    fn __repr__(&self) -> String {
        format!("Type({:?}", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __eq__(&self, other: Type) -> bool {
        self.0 == other.0
    }
}

fn const_from_py(g: &mut rust::Graph, val: &Bound<PyAny>) -> PyResult<Ref> {
    if let Ok(b) = val.extract::<bool>() {
        Ok(Ref(g.r#const(b)))
    } else if let Ok(float) = val.extract::<f64>() {
        Ok(Ref(g.r#const(float)))
    } else if let Ok(s) = val.extract::<String>() {
        Ok(Ref(g.push_symbol(s)))
    } else {
        return Err(exceptions::PyValueError::new_err(format!(
            "Cannot make constant out of a {}",
            val.get_type().name()?,
        )));
    }
}

#[pyfunction]
fn r#const(val: &Bound<PyAny>) -> PyResult<Ref> {
    graph::try_with_current(|g| const_from_py(g, val))
}

fn value_from_ref(g: &rust::Graph, scalar: Ref) -> PyResult<rust::layout::RefValue> {
    Ok(match g.type_of(scalar.0) {
        rust::Type::Float => rust::layout::RefValue::Scalar(scalar.0),
        rust::Type::Bool => rust::layout::RefValue::Bool(scalar.0),
        rust::Type::DateTime => rust::layout::RefValue::DateTime(scalar.0),
        rust::Type::Symbol => rust::layout::RefValue::Symbol(scalar.0),
        _ => {
            return Err(exceptions::PyException::new_err(format!(
                "cannot make RefValue out of {:?}",
                scalar.0
            )))
        }
    })
}

fn pythonize_ref_value(py: Python, val: rust::layout::RefValue) -> PyResult<PyObject> {
    Ok(match val {
        rust::layout::RefValue::Unit => PyNone::get_bound(py).to_owned().unbind().into(),
        rust::layout::RefValue::Scalar(s) => Ref(s).into_py(py),
        rust::layout::RefValue::Bool(s) => Ref(s).into_py(py),
        rust::layout::RefValue::DateTime(s) => Ref(s).into_py(py),
        rust::layout::RefValue::Symbol(e) => Ref(e).into_py(py),
        rust::layout::RefValue::Struct(fields) => {
            let dict = PyDict::new_bound(py);
            for (name, val) in fields {
                dict.set_item(name, pythonize_ref_value(py, val)?)?;
            }
            dict.unbind().into()
        }
        rust::layout::RefValue::List(l) => PyTuple::new_bound(
            py,
            l.into_iter()
                .map(|el| pythonize_ref_value(py, el))
                .collect::<PyResult<Vec<_>>>()?,
        )
        .unbind()
        .into(),
    })
}

fn depythonize_ref_value(
    g: &mut rust::Graph,
    obj: &Bound<PyAny>,
) -> PyResult<rust::layout::RefValue> {
    fn depythonize_inner(
        g: &mut rust::Graph,
        obj: &Bound<PyAny>,
    ) -> PyResult<rust::layout::RefValue> {
        if obj.is_none() {
            return Ok(rust::layout::RefValue::Unit);
        }

        if let Ok(scalar) = obj.extract::<Ref>() {
            return value_from_ref(g, scalar);
        }

        if let Ok(dict) = obj.downcast::<PyDict>() {
            let vals = dict
                .iter()
                .map(|(key, val)| Ok((key.extract::<String>()?, depythonize_inner(g, &val)?)))
                .collect::<PyResult<HashMap<String, rust::layout::RefValue>>>()?;
            return Ok(rust::layout::RefValue::Struct(vals));
        }

        if let Ok(list) = obj.downcast::<PyList>() {
            let vals = list
                .iter()
                .map(|val| depythonize_inner(g, &val))
                .collect::<PyResult<Vec<rust::layout::RefValue>>>()?;
            return Ok(rust::layout::RefValue::List(vals));
        }

        if let Ok(tuple) = obj.downcast::<PyTuple>() {
            let vals = tuple
                .iter()
                .map(|val| depythonize_inner(g, &val))
                .collect::<PyResult<Vec<rust::layout::RefValue>>>()?;
            return Ok(rust::layout::RefValue::List(vals));
        }

        if let Ok(scalar) = const_from_py(g, obj) {
            return value_from_ref(g, scalar);
        }

        Err(exceptions::PyTypeError::new_err(format!(
            "Cannot make {obj}, of type {}, into a RefValue",
            obj.get_type().name()?,
        )))
    }

    depythonize_inner(g, obj)
}

#[pyfunction]
fn read_metadata(file: &str) -> PyResult<HashMap<String, String>> {
    let file = std::fs::File::open(file)?;
    let metadata = rust::Graph::load_metadata(file).map_err(ToPyErr)?;
    Ok(metadata)
}

#[pyfunction]
fn read_graph(file: &str, initialize: Option<bool>) -> PyResult<Graph> {
    let initialize = initialize.unwrap_or(true);
    let file = std::fs::File::open(file)?;
    let inner = if initialize {
        rust::Graph::load(file)
    } else {
        rust::Graph::load_uninitialized(file)
    };
    Ok(Graph(Arc::new(Mutex::new(inner.map_err(ToPyErr)?))))
}

#[pyfunction]
fn read_fn(file: &str) -> PyResult<Function> {
    let file = std::fs::File::open(file)?;
    let inner = rust::Function::load(file).map_err(ToPyErr)?;
    Ok(Function {
        inner,
        original: None,
    })
}

#[pyfunction]
fn putative_layout(obj: &Bound<PyAny>) -> PyResult<Layout> {
    graph::try_with_current(|g| Ok(Layout(depythonize_ref_value(g, obj)?.putative_layout())))
}

#[pyfunction]
fn input(py: Python, name: String, layout: Option<Layout>) -> PyResult<PyObject> {
    if let Some(layout) = layout {
        graph::try_with_current(|g| pythonize_ref_value(py, g.input(name, layout.0)))
    } else {
        graph::try_with_current(|g| {
            pythonize_ref_value(py, g.input(name, rust::layout::Layout::Scalar))
        })
    }
}

#[pyfunction]
fn ret(val: &Bound<PyAny>, layout: Layout) -> PyResult<()> {
    graph::try_with_current(|g| {
        let val = depythonize_ref_value(g, val)?;
        Ok(g.output(val, layout.0).map_err(ToPyErr)?)
    })
}

#[pyfunction]
fn assert_(r#ref: &Bound<PyAny>, error_msg: String) -> PyResult<Ref> {
    let r#ref = Ref::make(r#ref)?;
    graph::try_with_current(|g| Ok(Ref(g.assert(r#ref.0, error_msg).map_err(ToPyErr)?)))
}
