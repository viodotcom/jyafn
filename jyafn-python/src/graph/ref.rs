use pyo3::exceptions;
use pyo3::prelude::*;

use crate::r#const;

use super::insert_in_current;

#[pyclass]
#[derive(Clone)]
pub struct Ref(pub(crate) rust::Ref);

impl Ref {
    pub fn make(obj: &Bound<PyAny>) -> PyResult<Ref> {
        if let Ok(r) = obj.extract() {
            Ok(r)
        } else {
            r#const(obj)
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

    fn __add__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Add, vec![self.0, other.0])
    }

    fn __radd__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Add, vec![other.0, self.0])
    }

    fn __sub__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Sub, vec![self.0, other.0])
    }

    fn __rsub__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Sub, vec![other.0, self.0])
    }

    fn __mul__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Mul, vec![self.0, other.0])
    }

    fn __rmul__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Mul, vec![other.0, self.0])
    }

    fn __truediv__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Div, vec![self.0, other.0])
    }

    fn __rtruediv__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Div, vec![other.0, self.0])
    }

    fn __floordiv__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        let divided = insert_in_current(rust::op::Div, vec![self.0, other.0])?;
        insert_in_current(rust::op::Call("floor".to_string()), vec![divided.0])
    }

    fn __rfloordiv__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        let divided = insert_in_current(rust::op::Div, vec![other.0, self.0])?;
        insert_in_current(rust::op::Call("floor".to_string()), vec![divided.0])
    }

    fn __mod__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Rem, vec![self.0, other.0])
    }

    fn __rmod__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Rem, vec![other.0, self.0])
    }

    fn __neg__(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Neg, vec![self.0])
    }

    fn __pos__(&self) -> Ref {
        Ref(self.0)
    }

    fn __pow__(&self, exponent: &Bound<PyAny>, _modulo: &Bound<PyAny>) -> PyResult<Ref> {
        let exponent = Ref::make(exponent)?;
        insert_in_current(rust::op::Call("powf".to_string()), vec![self.0, exponent.0])
    }

    fn __rpow__(&self, base: &Bound<PyAny>, _modulo: &Bound<PyAny>) -> PyResult<Ref> {
        let base = Ref::make(base)?;
        insert_in_current(rust::op::Call("powf".to_string()), vec![base.0, self.0])
    }

    fn __abs__(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Abs, vec![self.0])
    }

    fn __eq__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Eq(None), vec![self.0, other.0])
    }

    fn __lt__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Lt, vec![self.0, other.0])
    }

    fn __gt__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Gt, vec![self.0, other.0])
    }

    fn __le__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Le, vec![self.0, other.0])
    }

    fn __ge__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Ge, vec![self.0, other.0])
    }

    fn __invert__(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Not, vec![self.0])
    }

    fn __and__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::And, vec![self.0, other.0])
    }

    fn __rand__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::And, vec![other.0, self.0])
    }

    fn __or__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Or, vec![self.0, other.0])
    }

    fn __ror__(&self, other: &Bound<PyAny>) -> PyResult<Ref> {
        let other = Ref::make(other)?;
        insert_in_current(rust::op::Or, vec![other.0, self.0])
    }

    fn choose(&self, if_true: &Bound<PyAny>, if_false: &Bound<PyAny>) -> PyResult<Ref> {
        let if_true = Ref::make(if_true)?;
        let if_false = Ref::make(if_false)?;
        insert_in_current(rust::op::Choose, vec![self.0, if_true.0, if_false.0])
    }

    fn to_bool(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::ToBool, vec![self.0])
    }

    fn to_float(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::ToFloat, vec![self.0])
    }

    // Reimplementing pfuncs as methods allows us to take advantage of numpy
    // functionalities.

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

    // Datetime functions:

    fn timestamp(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("timestamp".to_string()), vec![self.0])
    }

    fn year(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("year".to_string()), vec![self.0])
    }

    fn month(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("month".to_string()), vec![self.0])
    }

    fn day(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("day".to_string()), vec![self.0])
    }

    fn hour(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("hour".to_string()), vec![self.0])
    }

    fn minute(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("minute".to_string()), vec![self.0])
    }

    fn second(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("second".to_string()), vec![self.0])
    }

    fn microsecond(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("microsecond".to_string()), vec![self.0])
    }

    fn weekday(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("weekday".to_string()), vec![self.0])
    }

    fn week(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("week".to_string()), vec![self.0])
    }

    fn dayofyear(&self) -> PyResult<Ref> {
        insert_in_current(rust::op::Call("dayofyear".to_string()), vec![self.0])
    }
}
