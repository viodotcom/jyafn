//! Implementation of all the operations allowed in [`Graph`]s. Just the subset of safe
//! operations are exposed in the public interface.

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
#[cfg(doc)]
use get_size::GetSize;
use std::fmt::Debug;
use std::panic::RefUnwindSafe;

use super::{FnError, Graph, Ref, Type};

/// The fundamental trait defining an operation in a computational graph.
#[typetag::serde(tag = "type")]
pub trait Op: 'static + DynClone + Debug + Send + Sync + RefUnwindSafe + Downcast {
    /// This function annotates the type of the output of this operation. It is required
    /// from the implementor that this function be idempotent.
    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type>;

    /// Renders the QBE code for this operation into a given function builder.
    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    );

    /// Checks if this operation is equal to another operation.
    fn is_eq(&self, other: &dyn Op) -> bool;

    /// Gets the total size in memory (stack + heap) that this operation takes.
    fn get_size(&self) -> usize;

    /// Attempts to evaluate the result of the application of this operation in compile
    /// time. Returns `None` if that is not possible. The default implementation always
    /// returns `None`.
    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        None
    }

    /// Whether this operation can be optimized away or not. If the method returns `true`,
    ///  the node this operation is associated with will never be removed from the graph,
    ///  even if it is unreachable. The default implementation always returns `false`.
    fn must_use(&self) -> bool {
        false
    }

    /// Checks whether this operation is correctly formed. This method can also be used
    /// to detect runtime errors in compilation time.
    #[allow(unused_variables)]
    fn is_illegal(&self, args: &[Ref]) -> bool {
        false
    }
}

dyn_clone::clone_trait_object!(Op);
impl_downcast!(Op);

/// Implements [`Op::is_eq`] for a given type.
///
/// # Usage
///
/// ```
/// impl Op for MyOp {
///     impl_is_eq! {}
/// }
/// ```
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

/// Implements [`Op::get_size`] for a given type that already implements [`GetSize`].
///
/// # Usage
///
/// ```
/// impl Op for MyOp {
///     impl_get_size! {}
/// }
/// ```
#[macro_export]
macro_rules! impl_get_size {
    () => {
        fn get_size(&self) -> usize {
            std::mem::size_of::<Self>()
        }
    };
}

/// Has the same effect of [`impl_is_eq`] and [`impl_get_size`]. Use this if you don't
/// care about creating a custom implementation for either of these functionalities.
#[macro_export]
macro_rules! impl_op {
    () => {
        $crate::impl_is_eq!();
        $crate::impl_get_size!();
    };
}

/// Generates an unique name for a QBE temporary, with the given prefix.
fn unique_for(v: qbe::Value, prefix: &str) -> String {
    let qbe::Value::Temporary(name) = v else {
        panic!("Can only get unique names for temporaries; got {v}")
    };

    format!("{prefix}_{name}")
}

/// Renders the call to create an [`FnError`] out of a static string in jyafn code.
pub(crate) fn render_return_error(func: &mut qbe::Function, error: qbe::Value) {
    let error_ptr = qbe::Value::Temporary("__error_ptr".to_string());
    func.assign_instr(
        error_ptr.clone(),
        qbe::Type::Long,
        qbe::Instr::Call(
            qbe::Value::Const(FnError::make_static as usize as u64),
            vec![(qbe::Type::Long, error)],
        ),
    );
    func.add_instr(qbe::Instr::Ret(Some(error_ptr)));
}
