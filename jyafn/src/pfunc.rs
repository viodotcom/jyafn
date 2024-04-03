use super::Type;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

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

#[derive(Debug, Clone, Copy)]
pub struct PFunc {
    fn_ptr: ThreadsafePointer,
    signature: &'static [Type],
    returns: Type,
    // const_eval: &'static Fn(&[f64]) -> Option<Vec<f64>>,
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
}

lazy_static! {
    static ref P_FUNCS: RwLock<HashMap<&'static str, PFunc>> = RwLock::new(init());
}

/// This function is unsafe because _anything_ can be passed as a function pointer,
/// including stuff that are note function. Its the caller responsibility to check that
/// `fn_ptr` is in fact a function pointer and that the arguments match the signature
/// given.
///
/// # Panics
///
/// This function panics if a pfunc of the given name has already been inscribed.
pub unsafe fn inscribe(name: &str, fn_ptr: *const (), signature: &[Type], returns: Type) {
    let mut guard = P_FUNCS.write().expect("poisoned");

    if guard.contains_key(name) {
        // This avoids poisoning the global lock.
        drop(guard);
        panic!("Function of name {name} already inscribed");
    }

    guard.insert(
        Box::leak(name.to_string().into_boxed_str()),
        PFunc {
            fn_ptr: ThreadsafePointer(fn_ptr),
            signature: Box::leak(signature.to_vec().into_boxed_slice()),
            returns,
        },
    );
}

pub fn get(name: &str) -> Option<PFunc> {
    let guard = P_FUNCS.read().expect("poisoned");
    guard.get(name).copied()
}

fn init() -> HashMap<&'static str, PFunc> {
    maplit::hashmap! {
        "sqrt" => PFunc {
            fn_ptr: ThreadsafePointer(sqrt as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "exp" => PFunc {
            fn_ptr: ThreadsafePointer(exp as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "ln" => PFunc {
            fn_ptr: ThreadsafePointer(ln as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "pow" => PFunc {
            fn_ptr: ThreadsafePointer(pow as *const ()),
            signature: &[Type::Float, Type::Float],
            returns: Type::Float,
        },
        "sin" => PFunc {
            fn_ptr: ThreadsafePointer(sin as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "cos" => PFunc {
            fn_ptr: ThreadsafePointer(cos as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "tan" => PFunc {
            fn_ptr: ThreadsafePointer(tan as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "asin" => PFunc {
            fn_ptr: ThreadsafePointer(asin as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "acos" => PFunc {
            fn_ptr: ThreadsafePointer(acos as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "atan" => PFunc {
            fn_ptr: ThreadsafePointer(atan as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "sinh" => PFunc {
            fn_ptr: ThreadsafePointer(sinh as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "cosh" => PFunc {
            fn_ptr: ThreadsafePointer(cosh as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "tanh" => PFunc {
            fn_ptr: ThreadsafePointer(tanh as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "asinh" => PFunc {
            fn_ptr: ThreadsafePointer(asinh as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "acosh" => PFunc {
            fn_ptr: ThreadsafePointer(acosh as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
        "atanh" => PFunc {
            fn_ptr: ThreadsafePointer(atanh as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
        },
    }
}

// fn call1(f: fn(f64) -> f64, args: &[f64]) -> Option<Vec<f64>> {
//     assert_eq!(args.len(), 1);
//     Some(vec![f(args[0])])
// }

fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

fn exp(x: f64) -> f64 {
    x.exp()
}

fn ln(x: f64) -> f64 {
    x.ln()
}

fn pow(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}

fn sin(x: f64) -> f64 {
    x.sin()
}

fn cos(x: f64) -> f64 {
    x.cos()
}

fn tan(x: f64) -> f64 {
    x.tan()
}

fn asin(x: f64) -> f64 {
    x.asin()
}

fn acos(x: f64) -> f64 {
    x.acos()
}

fn atan(x: f64) -> f64 {
    x.atan()
}

fn sinh(x: f64) -> f64 {
    x.sin()
}

fn cosh(x: f64) -> f64 {
    x.cos()
}

fn tanh(x: f64) -> f64 {
    x.tan()
}

fn asinh(x: f64) -> f64 {
    x.asin()
}

fn acosh(x: f64) -> f64 {
    x.acos()
}

fn atanh(x: f64) -> f64 {
    x.atan()
}
