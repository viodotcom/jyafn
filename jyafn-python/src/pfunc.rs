use super::{graph, Ref};

use pyo3::prelude::*;

pub fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
    macro_rules! pfunc1s {
        ($($f:ident),*) => { $(
            #[pyfunction]
            fn $f(x: &Bound<PyAny>) -> PyResult<Ref> {
                let x = Ref::make(x)?;
                graph::insert_in_current(rust::op::Call(stringify!($f).to_string()), vec![x.0])
            }

            m.add_function(wrap_pyfunction!($f, m)?)?;
        )* }
    }

    macro_rules! pfunc2s {
        ($($f:ident),*) => { $(
            #[pyfunction]
            fn $f(x: &Bound<PyAny>, y: &Bound<PyAny>) -> PyResult<Ref> {
                let x = Ref::make(x)?;
                let y = Ref::make(y)?;
                graph::insert_in_current(rust::op::Call(stringify!($f).to_string()), vec![x.0, y.0])
            }

            m.add_function(wrap_pyfunction!($f, m)?)?;
        )* }
    }

    pfunc1s! {
        // f64 -> f64
        floor, ceil, round, trunc,
        sqrt, exp, ln,
        sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, asinh, acosh, atanh,
        gamma, loggamma, factorial, rgamma, digamma,
        erf, erfc, norm, norm_inv,
        riemann_zeta,

        // dt -> f64
        timestamp,

        // f64 -> dt
        fromtimestamp
    }

    pfunc1s! {
        // f64 -> bool
        is_nan, is_finite, is_infinite
    }

    pfunc2s! {
        // f64, f64 -> f64
        powf, rem,
        beta, logbeta, gammainc, gammac, gammac_inv,
        besselj, bessely, besseli
    }

    // Misc that can't be solved with macros.
    m.add_function(wrap_pyfunction!(pow, m)?)?;

    Ok(())
}

#[pyfunction]
fn pow(base: &Bound<PyAny>, exponent: &Bound<PyAny>) -> PyResult<Ref> {
    let base = Ref::make(base)?;
    let exponent = Ref::make(exponent)?;
    graph::insert_in_current(rust::op::Call("powf".to_string()), vec![base.0, exponent.0])
}
