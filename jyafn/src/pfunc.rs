//! _Pure_ functions that operate on raw jyafn data.

use chrono::prelude::*;
use lazy_static::lazy_static;
use special_fun::FloatSpecial;
use std::collections::HashMap;
use std::ops::Rem;
use std::sync::RwLock;

use super::{utils, Error, Type};

/// A pointer that you pinky-promisse to be thread-safe (i.e., the reference behind it is
/// `Send` and `Sync`). Only use with function pointers and _nothing_ else (this last
/// bound is still difficult to express in the Rust type system).
#[derive(Debug, Clone, Copy)]
struct ThreadsafePointer(*const ());

unsafe impl Send for ThreadsafePointer {}
unsafe impl Sync for ThreadsafePointer {}

impl From<ThreadsafePointer> for *const () {
    fn from(ptr: ThreadsafePointer) -> *const () {
        ptr.0
    }
}

/// Wraps a closure that does compile-time evaluation for a pure function.
#[derive(Clone, Copy)]
pub(crate) struct ConstEval(pub(crate) &'static (dyn Send + Sync + Fn(&[f64]) -> Option<f64>));

impl ConstEval {
    /// No compile-time evaluation will be done.
    fn no_eval() -> ConstEval {
        ConstEval(&|_| None)
    }

    /// Compile-time evalue for `fn(f64) -> f64`.
    fn call1(f: fn(f64) -> f64) -> ConstEval {
        let closure = move |args: &[f64]| {
            assert_eq!(args.len(), 1);
            Some(f(args[0]))
        };

        ConstEval(Box::leak(Box::new(closure)))
    }

    /// Compile-time evalue for `fn(f64, f64) -> f64`.
    fn call2(f: fn(f64, f64) -> f64) -> ConstEval {
        let closure = move |args: &[f64]| {
            assert_eq!(args.len(), 2);
            Some(f(args[0], args[1]))
        };

        ConstEval(Box::leak(Box::new(closure)))
    }
}

impl std::fmt::Debug for ConstEval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConstEval")
    }
}

/// A pure function.
///
/// Pure functions should always
/// 1. Yield the same result for the same inputs in all contexts.
/// 2. Never fail. This includes panics, since jyafn is an FFI boundary and panics
///    through FFI boundaries are undefined behavior.
///
///
#[derive(Debug, Clone, Copy)]
pub struct PFunc {
    /// The raw function pointer to the function.
    fn_ptr: ThreadsafePointer,
    /// The input types of the function.
    signature: &'static [Type],
    /// The return type of the function.    /// The return type of the function
    returns: Type,
    /// Provides compile-time evaluation behavior.
    pub(crate) const_eval: ConstEval,
}

impl PFunc {
    /// The input types of the function.
    pub fn signature(self) -> &'static [Type] {
        self.signature
    }

    /// The return type of the function
    pub fn returns(self) -> Type {
        self.returns
    }

    /// The memory address of the function. This is what will be harcoded in the jyafn
    /// code.
    pub fn location(self) -> usize {
        self.fn_ptr.0 as usize
    }

    /// Creates a [`PFunc`] for a `fn(f64) -> f64`.
    fn call1(f: fn(f64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
            const_eval: ConstEval::call1(f),
        }
    }

    /// Creates a [`PFunc`] for a `fn(f64, 64) -> f64`.
    fn call2(f: fn(f64, f64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float, Type::Float],
            returns: Type::Float,
            const_eval: ConstEval::call2(f),
        }
    }

    /// Creates a [`PFunc`] for a `fn(f64) -> bool`.
    fn call_bool_to_f64(f: fn(f64) -> bool) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float],
            returns: Type::Bool,
            const_eval: ConstEval::no_eval(),
        }
    }

    /// Creates a [`PFunc`] for a `fn(i64) -> f64`, where the input is a timestamp.
    pub fn call_dt_to_f64(f: fn(i64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::DateTime],
            returns: Type::Float,
            const_eval: ConstEval::no_eval(),
        }
    }

    /// Creates a [`PFunc`] for a `fn(f64) -> i64`, where the output is a timestamp.
    pub fn call_f64_to_dt(f: fn(f64) -> i64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float],
            returns: Type::DateTime,
            const_eval: ConstEval::no_eval(),
        }
    }
}

lazy_static! {
    /// All the known [`PFunc`]s.
    static ref P_FUNCS: RwLock<HashMap<&'static str, PFunc>> = RwLock::new(init());
}

/// Inscribes a new pure function.
///
/// # Safety
///
/// This function is unsafe because _anything_ can be passed as a function pointer,
/// including stuff that are not a function. Its the caller responsibility to check that
/// `fn_ptr` is in fact a function pointer and that the arguments match the signature
/// given and that the function that is being supplied actually obeys all the expectations
/// on a pure function (see [`PFunc`] for the requirements.)
///
/// # Panics
///
/// This function panics if a pfunc of the given name has already been inscribed.
pub unsafe fn inscribe(
    name: &str,
    fn_ptr: *const (),
    signature: &[Type],
    returns: Type,
) -> Result<(), Error> {
    let mut guard = P_FUNCS.write().expect("poisoned");

    if guard.contains_key(name) {
        // This avoids poisoning the global lock.
        drop(guard);
        return Err("Function of name {name} already inscribed"
            .to_string()
            .into());
    }

    guard.insert(
        Box::leak(name.to_string().into_boxed_str()),
        PFunc {
            fn_ptr: ThreadsafePointer(fn_ptr),
            signature: Box::leak(signature.to_vec().into_boxed_slice()),
            returns,
            const_eval: ConstEval::no_eval(),
        },
    );

    Ok(())
}

/// Gets a pure function by name, returning `None` if none is found.
pub fn get(name: &str) -> Option<PFunc> {
    let guard = P_FUNCS.read().expect("poisoned");
    guard.get(name).copied()
}

/// Initalizes the [`P_FUNCS`] static with the standard pure function provided by jyafn.
#[allow(unstable_name_collisions)]
fn init() -> HashMap<&'static str, PFunc> {
    let mut map = HashMap::new();

    macro_rules! pfuncs_f64 {
        ($($method:ident : $($f:ident),*);*) => { $($(
            map.insert(stringify!($f), PFunc::$method(f64::$f));
        )*)* }
    }

    pfuncs_f64! {
        call1:
            floor, ceil, round, trunc,
            sqrt, exp, ln, ln_1p, exp_m1,
            sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, asinh, acosh, atanh,
            gamma, loggamma, factorial, rgamma, digamma,
            erf, erfc, norm, norm_inv,
            riemann_zeta;
        call2:
            powf, rem, atan2,
            beta, logbeta, gammainc, gammac, gammac_inv,
            besselj, bessely, besseli;
        call_bool_to_f64:
            is_nan, is_finite, is_infinite
    }

    macro_rules! pfuncs {
        ($($method:ident : $($f:ident),*);*) => { $($(
            map.insert(stringify!($f), PFunc::$method($f));
        )*)* }
    }

    pfuncs! {
        call_f64_to_dt:
            fromtimestamp;
        call_dt_to_f64:
            timestamp, year, month, day, hour, minute, second, microsecond,
            weekday, week, dayofyear
    }

    map
}

fn fromtimestamp(x: f64) -> i64 {
    (x * 1e6) as i64
}

fn timestamp(dt: i64) -> f64 {
    dt as f64 / 1e6
}

fn year(dt: i64) -> f64 {
    utils::int_to_datetime(dt).year() as f64
}

fn month(dt: i64) -> f64 {
    utils::int_to_datetime(dt).month() as f64
}

fn day(dt: i64) -> f64 {
    utils::int_to_datetime(dt).day() as f64
}

fn hour(dt: i64) -> f64 {
    utils::int_to_datetime(dt).hour() as f64
}

fn minute(dt: i64) -> f64 {
    utils::int_to_datetime(dt).minute() as f64
}

fn second(dt: i64) -> f64 {
    utils::int_to_datetime(dt).second() as f64
}

fn microsecond(dt: i64) -> f64 {
    utils::int_to_datetime(dt).timestamp_subsec_micros() as f64
}

fn weekday(dt: i64) -> f64 {
    utils::int_to_datetime(dt).weekday() as i64 as f64
}

fn week(dt: i64) -> f64 {
    utils::int_to_datetime(dt).iso_week().week() as i64 as f64
}

fn dayofyear(dt: i64) -> f64 {
    utils::int_to_datetime(dt).ordinal() as f64
}
