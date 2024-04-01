extern crate jyafn as rust;

mod dataset;
mod layout;
mod pfunc;

use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyNone, PyString, PyTuple};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[pymodule]
fn jyafn(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Graph>()?;
    m.add_class::<Ref>()?;
    m.add_class::<Type>()?;
    m.add_class::<Layout>()?;
    m.add_class::<Function>()?;
    m.add_function(wrap_pyfunction!(current_graph, m)?)?;
    m.add_function(wrap_pyfunction!(r#const, m)?)?;
    m.add_function(wrap_pyfunction!(input, m)?)?;
    m.add_function(wrap_pyfunction!(list_input, m)?)?;
    m.add_function(wrap_pyfunction!(enum_input, m)?)?;
    m.add_function(wrap_pyfunction!(scalar_ret, m)?)?;
    m.add_function(wrap_pyfunction!(list_ret, m)?)?;
    m.add_function(wrap_pyfunction!(ret, m)?)?;

    pfunc::init(m)?;
    dataset::init(m)?;

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
struct Graph(Arc<Mutex<rust::Graph>>);

#[pymethods]
impl Graph {
    #[new]
    fn new(name: Option<String>) -> Graph {
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
        Ok(Function(
            self.0
                .lock()
                .expect("poisoned")
                .compile()
                .map_err(ToPyErr)?,
        ))
    }
}

thread_local! {
    static CONTEXT: RefCell<Vec<Graph>> = RefCell::new(vec![Graph::new(Some("main".to_string()))]);
}

#[pyfunction]
fn current_graph() -> PyResult<Graph> {
    CONTEXT.with_borrow(|context| {
        context
            .last()
            .cloned()
            .ok_or_else(|| exceptions::PyException::new_err("no current graph found"))
    })
}

fn try_with_current<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce(&mut rust::Graph) -> PyResult<T>,
{
    let current = current_graph()?;
    let mut lock = current.0.lock().expect("poisoned");
    f(&mut *lock)
}

fn with_current<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce(&mut rust::Graph) -> T,
{
    try_with_current(|g| Ok(f(g)))
}

fn insert_in_current<O: rust::Op>(op: O, args: Vec<rust::Ref>) -> PyResult<Ref> {
    try_with_current(|g| Ok(Ref(g.insert(op, args).map_err(ToPyErr)?)))
}

#[pyclass]
#[derive(Clone)]
struct Type(rust::Type);

#[pyclass]
#[derive(Clone)]
struct Ref(rust::Ref);

impl Ref {
    fn make(py: Python, obj: PyObject) -> PyResult<Ref> {
        if let Ok(r) = obj.extract(py) {
            Ok(r)
        } else {
            r#const(py, obj)
        }
    }
}

#[pymethods]
impl Ref {
    fn __repr__(&self) -> String {
        match self.0 {
            rust::Ref::Input(input_id) => format!("Ref(input={input_id})"),
            rust::Ref::Const(ty, rendered) => {
                format!("Ref(ty={ty:?}, rendered={rendered})",)
            }
            rust::Ref::Node(node_id) => format!("Ref(node={node_id})"),
        }
    }

    fn __bool__(&self) -> PyResult<bool> {
        Err(exceptions::PyTypeError::new_err(
            "Cannot assert the truthiness of a Ref",
        ))
    }

    fn __add__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Add, vec![self.0, other.0])
    }

    fn __radd__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Add, vec![other.0, self.0])
    }

    fn __sub__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Sub, vec![self.0, other.0])
    }

    fn __rsub__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Sub, vec![other.0, self.0])
    }

    fn __mul__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Mul, vec![self.0, other.0])
    }

    fn __rmul__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Mul, vec![other.0, self.0])
    }

    fn __div__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Div, vec![self.0, other.0])
    }

    fn __rdiv__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Div, vec![other.0, self.0])
    }

    fn __neg__(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Neg, vec![self.0])
    }

    fn __pos__(&self) -> Ref {
        Ref(self.0)
    }

    fn __pow__(&self, py: Python, exponent: PyObject, _modulo: PyObject) -> PyResult<Ref> {
        let exponent = Ref::make(py, exponent)?;
        insert_in_current(rust::op::Call("pow".to_string()), vec![self.0, exponent.0])
    }

    fn __rpow__(&self, py: Python, base: PyObject, _modulo: PyObject) -> PyResult<Ref> {
        let base = Ref::make(py, base)?;
        insert_in_current(rust::op::Call("pow".to_string()), vec![base.0, self.0])
    }

    fn __abs__(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Abs, vec![self.0])
    }

    fn __eq__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Eq, vec![self.0, other.0])
    }

    fn __lt__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Lt, vec![self.0, other.0])
    }

    fn __gt__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Gt, vec![self.0, other.0])
    }

    fn __le__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Le, vec![self.0, other.0])
    }

    fn __ge__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Ge, vec![self.0, other.0])
    }

    fn __invert__(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Not, vec![self.0])
    }

    fn __and__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::And, vec![self.0, other.0])
    }

    fn __rand__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::And, vec![other.0, self.0])
    }

    fn __or__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Or, vec![self.0, other.0])
    }

    fn __ror__(&self, py: Python, other: PyObject) -> PyResult<Ref> {
        let other = Ref::make(py, other)?;
        insert_in_current(rust::op::Or, vec![other.0, self.0])
    }

    fn choose(&self, py: Python, if_true: PyObject, if_false: PyObject) -> PyResult<Ref> {
        let if_true = Ref::make(py, if_true)?;
        let if_false = Ref::make(py, if_false)?;
        insert_in_current(rust::op::Choose, vec![self.0, if_true.0, if_false.0])
    }

    fn to_bool(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::ToBool, vec![self.0])
    }

    fn to_float(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::ToFloat, vec![self.0])
    }

    // Reimplementing pfuncs as methods allows us to take advantage of numpy's ufuncs.

    fn sqrt(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("sqrt".to_string()), vec![self.0])
    }

    fn exp(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("exp".to_string()), vec![self.0])
    }

    fn ln(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("ln".to_string()), vec![self.0])
    }

    /// To make numpy happy.
    fn log(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("ln".to_string()), vec![self.0])
    }

    fn sin(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("sin".to_string()), vec![self.0])
    }

    fn cos(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("cos".to_string()), vec![self.0])
    }

    fn tan(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("tan".to_string()), vec![self.0])
    }

    fn asin(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("asin".to_string()), vec![self.0])
    }

    fn acos(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("acos".to_string()), vec![self.0])
    }

    fn atan(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("atan".to_string()), vec![self.0])
    }

    fn sinh(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("sinh".to_string()), vec![self.0])
    }

    fn cosh(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("cosh".to_string()), vec![self.0])
    }

    fn tanh(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("tanh".to_string()), vec![self.0])
    }

    fn asinh(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("asinh".to_string()), vec![self.0])
    }

    fn acosh(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("acosh".to_string()), vec![self.0])
    }

    fn atanh(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("atanh".to_string()), vec![self.0])
    }
}

#[pyfunction]
fn r#const(py: Python, val: PyObject) -> PyResult<Ref> {
    if let Ok(float) = val.extract::<f64>(py) {
        with_current(|g| Ref(g.r#const(float)))
    } else if let Ok(b) = val.extract::<bool>(py) {
        with_current(|g| Ref(g.r#const(b)))
    } else {
        return Err(exceptions::PyValueError::new_err(format!(
            "Cannot make constant out of a {}",
            val.bind(py).get_type().name()?,
        )));
    }
}

fn pythonize_ref_value(py: Python, val: rust::layout::RefValue) -> PyResult<PyObject> {
    Ok(match val {
        rust::layout::RefValue::Unit => PyNone::get_bound(py).to_owned().unbind().into(),
        rust::layout::RefValue::Scalar(s) => Ref(s).into_py(py),
        rust::layout::RefValue::Struct(fields) => {
            let dict = PyDict::new_bound(py);
            for (name, val) in fields {
                dict.set_item(name, pythonize_ref_value(py, val)?)?;
            }
            dict.unbind().into()
        }
        rust::layout::RefValue::Enum(e) => Ref(e).into_py(py),
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

fn depythonize_ref_value(py: Python, obj: &PyAny) -> PyResult<rust::layout::RefValue> {
    if obj.is_none() {
        return Ok(rust::layout::RefValue::Unit);
    }

    if let Ok(scalar) = obj.extract::<Ref>() {
        return Ok(rust::layout::RefValue::Scalar(scalar.0));
    }

    if let Ok(dict) = obj.downcast::<PyDict>() {
        let vals = dict
            .iter()
            .map(|(key, val)| Ok((key.extract::<String>()?, depythonize_ref_value(py, val)?)))
            .collect::<PyResult<HashMap<String, rust::layout::RefValue>>>()?;
        return Ok(rust::layout::RefValue::Struct(vals));
    }

    if let Ok(list) = obj.downcast::<PyList>() {
        let vals = list
            .iter()
            .map(|val| depythonize_ref_value(py, val))
            .collect::<PyResult<Vec<rust::layout::RefValue>>>()?;
        return Ok(rust::layout::RefValue::List(vals));
    }

    if let Ok(tuple) = obj.downcast::<PyTuple>() {
        let vals = tuple
            .iter()
            .map(|val| depythonize_ref_value(py, val))
            .collect::<PyResult<Vec<rust::layout::RefValue>>>()?;
        return Ok(rust::layout::RefValue::List(vals));
    }

    Err(exceptions::PyTypeError::new_err(format!(
        "Cannot make {obj} into a RefValue"
    )))
}

#[pyfunction]
fn input(py: Python, name: String, layout: Option<Layout>) -> PyResult<PyObject> {
    if let Some(layout) = layout {
        try_with_current(|g| pythonize_ref_value(py, g.input(name, layout.0)))
    } else {
        with_current(|g| Ref(g.scalar_input(name)).into_py(py))
    }
}

#[pyfunction]
fn list_input(name: String, size: usize) -> PyResult<Vec<Ref>> {
    with_current(|g| {
        g.vec_input(name, size)
            .into_iter()
            .map(|r| Ref(r))
            .collect()
    })
}

#[pyfunction]
fn enum_input(name: String, options: Vec<String>) -> PyResult<Ref> {
    with_current(|g| Ref(g.enum_input(name, options)))
}

#[pyfunction]
fn scalar_ret(py: Python, r#ref: PyObject) -> PyResult<()> {
    let r#ref = Ref::make(py, r#ref)?;
    with_current(|g| g.scalar_output(r#ref.0))
}

#[pyfunction]
fn list_ret(refs: Vec<Ref>) -> PyResult<()> {
    with_current(|g| g.slice_output(&refs.into_iter().map(|r| r.0).collect::<Vec<_>>()))
}

#[pyfunction]
fn ret(py: Python, val: PyObject, layout: Layout) -> PyResult<()> {
    let val = depythonize_ref_value(py, val.bind(py).as_gil_ref())?;
    try_with_current(|g| Ok(g.output(val, layout.0).map_err(ToPyErr)?))
}

#[pyclass]
struct Function(rust::Function);

#[pymethods]
impl Function {
    #[getter]
    fn input_size(&self) -> usize {
        self.0.input_size()
    }

    #[getter]
    fn output_size(&self) -> usize {
        self.0.output_size()
    }

    #[getter]
    fn input_layout(&self) -> Layout {
        Layout(self.0.input_layout().clone())
    }

    #[getter]
    fn output_layout(&self) -> Layout {
        Layout(self.0.output_layout().clone())
    }

    #[getter]
    fn fn_ptr(&self) -> usize {
        self.0.fn_ptr() as *const () as usize
    }

    #[staticmethod]
    pub fn load(bytes: &[u8]) -> PyResult<Function> {
        Ok(Function(rust::Function::load(bytes).map_err(ToPyErr)?))
    }

    pub fn dump<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let data = self.0.graph().dump();
        let leaked = Box::leak(data.into_boxed_slice());
        // Safety: leaking the box from rust and giving it to Python. Therefore, no
        // double free.
        unsafe { PyBytes::bound_from_ptr(py, leaked.as_ptr(), leaked.len()) }
    }

    pub fn to_json(&self) -> String {
        self.0.graph().to_json()
    }

    fn get_graph(&self) -> Graph {
        Graph(Arc::new(Mutex::new(self.0.graph().clone())))
    }

    fn eval_raw(&self, args: &[u8]) -> PyResult<Vec<u8>> {
        Ok(self
            .0
            .eval_raw(args)
            .map_err(ToPyErr)
            .map(|o| o.into_vec())?)
    }

    fn eval(&self, val: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        Ok(self
            .0
            .eval_with_decoder(
                &crate::layout::Obj(val.clone()),
                crate::layout::PyDecoder(val.py()),
            )
            .map_err(ToPyErr)?)
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
        let rust::layout::Layout::Struct(s) = self.0.input_layout() else {
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

#[pyclass]
#[derive(Clone)]
struct Layout(rust::layout::Layout);

#[pymethods]
impl Layout {
    fn __repr__(&self) -> String {
        format!(
            "Layout({})",
            serde_json::to_string(&self.0).expect("can always serialize")
        )
    }

    fn is_unit(&self) -> bool {
        self.0 == rust::layout::Layout::Unit
    }

    fn is_scalar(&self) -> bool {
        self.0 == rust::layout::Layout::Scalar
    }

    fn struct_keys(&self, py: Python) -> PyResult<PyObject> {
        let rust::layout::Layout::Struct(s) = &self.0 else {
            return Ok(pyo3::types::PyNone::get_bound(py).to_object(py));
        };

        let list = pyo3::types::PyList::new_bound(py, s.0.iter().map(|(name, _)| name.clone()));

        Ok(list.to_object(py))
    }

    fn as_enum(&self) -> Option<Vec<String>> {
        let rust::layout::Layout::Enum(s) = &self.0 else {
            return None;
        };

        Some(s.clone())
    }

    #[staticmethod]
    fn unit() -> Layout {
        Layout(rust::layout::Layout::Unit)
    }

    #[staticmethod]
    fn scalar() -> Layout {
        Layout(rust::layout::Layout::Scalar)
    }

    #[staticmethod]
    fn r#enum(options: Vec<String>) -> Layout {
        Layout(rust::layout::Layout::Enum(options))
    }

    #[staticmethod]
    fn list_of(element: &Layout, size: usize) -> Layout {
        Layout(rust::layout::Layout::List(
            Box::new(element.0.clone()),
            size,
        ))
    }

    #[staticmethod]
    fn struct_of(fields: &Bound<'_, PyDict>) -> PyResult<Layout> {
        let fields = fields
            .iter()
            .map(|(key, value)| Ok((key.extract::<String>()?, value.extract::<Layout>()?.0)))
            .collect::<PyResult<Vec<(String, rust::layout::Layout)>>>()?;

        Ok(Layout(rust::layout::Layout::Struct(rust::layout::Struct(
            fields,
        ))))
    }
}
