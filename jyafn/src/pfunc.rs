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

    pub fn call1(f: fn(f64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
            const_eval: ConstEval::call1(f),
        }
    }

    pub fn call2(f: fn(f64, f64) -> f64) -> PFunc {
        PFunc {
            fn_ptr: ThreadsafePointer(f as *const ()),
            signature: &[Type::Float],
            returns: Type::Float,
            const_eval: ConstEval::call2(f),
        }
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
            const_eval: ConstEval::no_eval(),
        },
    );
}

pub fn get(name: &str) -> Option<PFunc> {
    let guard = P_FUNCS.read().expect("poisoned");
    guard.get(name).copied()
}

fn init() -> HashMap<&'static str, PFunc> {
    maplit::hashmap! {
        "sqrt" => PFunc::call1(sqrt),
        "exp" => PFunc::call1(exp),
        "ln" => PFunc::call1(ln),
        "pow" => PFunc::call2(pow),
        "sin" => PFunc::call1(sin),
        "cos" => PFunc::call1(cos),
        "tan" => PFunc::call1(tan),
        "asin" => PFunc::call1(asin),
        "acos" => PFunc::call1(acos),
        "atan" => PFunc::call1(atan),
        "sinh" => PFunc::call1(sinh),
        "cosh" => PFunc::call1(cosh),
        "tanh" => PFunc::call1(tanh),
        "asinh" => PFunc::call1(asinh),
        "acosh" => PFunc::call1(acosh),
        "atanh" => PFunc::call1(atanh),
    }
}

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
