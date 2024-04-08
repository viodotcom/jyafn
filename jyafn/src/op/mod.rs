#![allow(unused_variables)]

mod arithmetic;
mod call;
mod compare;
mod convert;
mod logic;
mod mapping;

pub use arithmetic::*;
pub use call::*;
pub use compare::*;
pub use convert::*;
pub use logic::*;

pub(crate) use mapping::*;

use downcast_rs::{impl_downcast, Downcast};
use std::fmt::Debug;
use std::panic::RefUnwindSafe;

use super::{Graph, Ref, Type};

#[typetag::serde(tag = "type")]
pub trait Op: 'static + Debug + Send + Sync + RefUnwindSafe + Downcast {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type>;
    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
    );

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

impl_downcast!(Op);

fn unique_for(v: qbe::Value, prefix: &str) -> String {
    let qbe::Value::Temporary(name) = v else {
        panic!("Can only get unique names for temporaries; got {v}")
    };

    format!("{prefix}_{name}")
}
