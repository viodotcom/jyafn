use super::{insert_in_current, Ref};

use pyo3::prelude::*;

pub fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sqrt, m)?)?;
    m.add_function(wrap_pyfunction!(exp, m)?)?;
    m.add_function(wrap_pyfunction!(ln, m)?)?;
    m.add_function(wrap_pyfunction!(pow, m)?)?;

    m.add_function(wrap_pyfunction!(sin, m)?)?;
    m.add_function(wrap_pyfunction!(cos, m)?)?;
    m.add_function(wrap_pyfunction!(tan, m)?)?;
    m.add_function(wrap_pyfunction!(asin, m)?)?;
    m.add_function(wrap_pyfunction!(acos, m)?)?;
    m.add_function(wrap_pyfunction!(atan, m)?)?;
    m.add_function(wrap_pyfunction!(sinh, m)?)?;
    m.add_function(wrap_pyfunction!(cosh, m)?)?;
    m.add_function(wrap_pyfunction!(tanh, m)?)?;
    m.add_function(wrap_pyfunction!(asinh, m)?)?;
    m.add_function(wrap_pyfunction!(acosh, m)?)?;
    m.add_function(wrap_pyfunction!(atanh, m)?)?;

    Ok(())
}

#[pyfunction]
fn sqrt(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("sqrt".to_string()), vec![x.0])
}

#[pyfunction]
fn exp(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("exp".to_string()), vec![x.0])
}

#[pyfunction]
fn ln(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("ln".to_string()), vec![x.0])
}

#[pyfunction]
fn pow(py: Python, base: PyObject, exponent: PyObject) -> PyResult<Ref> {
    let base = Ref::make(py, base)?;
    let exponent = Ref::make(py, exponent)?;
    insert_in_current(rust::op::Call("pow".to_string()), vec![base.0, exponent.0])
}

#[pyfunction]
fn sin(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("sin".to_string()), vec![x.0])
}

#[pyfunction]
fn cos(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("cos".to_string()), vec![x.0])
}

#[pyfunction]
fn tan(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("tan".to_string()), vec![x.0])
}

#[pyfunction]
fn asin(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("asin".to_string()), vec![x.0])
}

#[pyfunction]
fn acos(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("acos".to_string()), vec![x.0])
}

#[pyfunction]
fn atan(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("atan".to_string()), vec![x.0])
}

#[pyfunction]
fn sinh(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("sinh".to_string()), vec![x.0])
}

#[pyfunction]
fn cosh(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("cosh".to_string()), vec![x.0])
}

#[pyfunction]
fn tanh(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("tanh".to_string()), vec![x.0])
}

#[pyfunction]
fn asinh(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("asinh".to_string()), vec![x.0])
}

#[pyfunction]
fn acosh(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("acosh".to_string()), vec![x.0])
}

#[pyfunction]
fn atanh(py: Python, x: PyObject) -> PyResult<Ref> {
    let x = Ref::make(py, x)?;
    insert_in_current(rust::op::Call("atanh".to_string()), vec![x.0])
}
