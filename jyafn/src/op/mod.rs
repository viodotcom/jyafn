#![allow(unused_variables)]

mod arithmetic;
mod call;
mod compare;
mod convert;
mod list;
mod logic;
mod mapping;
mod resource;

pub use arithmetic::*;
pub use call::*;
pub use compare::*;
pub use convert::*;
pub use logic::*;

pub(crate) use list::*;
pub(crate) use mapping::*;
pub(crate) use resource::*;

use downcast_rs::{impl_downcast, Downcast};
use dyn_clone::DynClone;
use std::fmt::Debug;
use std::panic::RefUnwindSafe;

use super::{FnError, Graph, Ref, Type};

#[typetag::serde(tag = "type")]
pub trait Op: 'static + DynClone + Debug + Send + Sync + RefUnwindSafe + Downcast {
    /// This function annotates the type of the output of this operation. It is required
    /// from the implementor that this function be idempotent.
    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type>;
    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    );
    fn is_eq(&self, other: &dyn Op) -> bool;
    fn get_size(&self) -> usize;

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        None
    }

    fn must_use(&self) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn is_illegal(&self, args: &[Ref]) -> bool {
        false
    }
}

dyn_clone::clone_trait_object!(Op);
impl_downcast!(Op);

#[macro_export]
macro_rules! impl_is_eq {
    () => {
        fn is_eq(&self, other: &dyn Op) -> bool {
            if let Some(same) = other.as_any().downcast_ref::<Self>() {
                self == same
            } else {
                false
            }
        }
    };
}

#[macro_export]
macro_rules! impl_get_size {
    () => {
        fn get_size(&self) -> usize {
            std::mem::size_of::<Self>()
        }
    };
}

#[macro_export]
macro_rules! impl_op {
    () => {
        $crate::impl_is_eq!();
        $crate::impl_get_size!();
    };
}

fn unique_for(v: qbe::Value, prefix: &str) -> String {
    let qbe::Value::Temporary(name) = v else {
        panic!("Can only get unique names for temporaries; got {v}")
    };

    format!("{prefix}_{name}")
}

pub(crate) fn render_return_error(func: &mut qbe::Function, error: qbe::Value) {
    let error_ptr = qbe::Value::Temporary("__error_ptr".to_string());
    func.assign_instr(
        error_ptr.clone(),
        qbe::Type::Long,
        qbe::Instr::Call(
            qbe::Value::Const(FnError::make_static as u64),
            vec![(qbe::Type::Long, error)],
        ),
    );
    func.add_instr(qbe::Instr::Ret(Some(error_ptr)));
}
