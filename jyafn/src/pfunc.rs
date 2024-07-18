use chrono::prelude::*;
use lazy_static::lazy_static;
use special_fun::FloatSpecial;
use std::collections::HashMap;
use std::ops::Rem;
use std::sync::RwLock;

use super::{utils, Error, Type};

/// Only use with function pointers and _nothing_ else.
#[derive(Debug, Clone, Copy)]
struct ThreadsafePointer(*const ());

unsafe impl Send for ThreadsafePointer {}
unsafe impl Sync for ThreadsafePointer {}

impl From<ThreadsafePointer> for *const () {
    fn from(ptr: ThreadsafePointer) -> *const () {
        ptr.0
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ConstEval(pub(crate) &'static (dyn Send + Sync + Fn(&[f64]) -> Option<f64>));

impl ConstEval {
    fn no_eval() -> ConstEval {
        ConstEval(&|_| None)
    }

    fn call1(f: fn(f64) -> f64) -> ConstEval {
        let closure = move |args: &[f64]| {
            assert_eq!(args.len(), 1);
            Some(f(args[0]))
        };

        ConstEval(Box::leak(Box::new(closure)))
    }

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

#[derive(Debug, Clone, Copy)]
pub struct PFunc {
    fn_ptr: ThreadsafePointer,
    signature: &'static [Type],
    returns: Type,
    pub(crate) const_eval: ConstEval,
}

impl PFunc {
    pub fn signature(self) -> &'static [Type] {
        self.signature
    }

    pub fn returns(self) -> Type {
        self.returns
    }

    pub fn location(self) -> usize {
        self.fn_ptr.0 as usize
    }

    fn call1(f: fn(f64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
            const_eval: ConstEval::call1(f),
        }
    }

    fn call2(f: fn(f64, f64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float, Type::Float],
            returns: Type::Float,
            const_eval: ConstEval::call2(f),
        }
    }

    fn call_bool_to_f64(f: fn(f64) -> bool) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float],
            returns: Type::Bool,
            const_eval: ConstEval::no_eval(),
        }
    }

    pub fn call_dt_to_f64(f: fn(i64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::DateTime],
            returns: Type::Float,
            const_eval: ConstEval::no_eval(),
        }
    }

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
    static ref P_FUNCS: RwLock<HashMap<&'static str, PFunc>> = RwLock::new(init());
}

/// # Safety
///
/// This function is unsafe because _anything_ can be passed as a function pointer,
/// including stuff that are not a function. Its the caller responsibility to check that
/// `fn_ptr` is in fact a function pointer and that the arguments match the signature
/// given.
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

pub fn get(name: &str) -> Option<PFunc> {
    let guard = P_FUNCS.read().expect("poisoned");
    guard.get(name).copied()
}

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
