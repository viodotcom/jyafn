#![allow(unexpected_cfgs)] // ... while we don't know what to do with map-reduce

#[cfg(not(target_pointer_width = "64"))]
compile_error!("Currently `jyafn` only works in 64-bit atchitectures");

pub mod r#const;
pub mod layout;
pub mod mapping;
pub mod op;
pub mod pfunc;
pub mod utils;

mod function;
mod graph;

#[cfg(feature = "map-reduce")]
pub use dataset::Dataset;
pub use function::{Function, FunctionData, RawFn};
pub use graph::{Graph, IndexedList, Node, Ref, Type};
pub use op::Op;
pub use r#const::Const;

use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display},
    process::ExitStatus,
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("cannot apply {0:?} on {1:?}")]
    Type(Box<dyn Op>, Vec<Type>),
    #[error("reference for {0:?} has already been defined")]
    AlreadyDefined(String),
    #[error("{0}")]
    Io(std::io::Error),
    #[error("found illegal instruction: {0}")]
    IllegalInstruction(String),
    #[error("qbe failed with {status}: {err}")]
    Qbe { status: ExitStatus, err: String },
    #[error("assembler failed with status {status}: {err}")]
    Assembler { status: ExitStatus, err: String },
    #[error("linker failed with status {status}: {err}")]
    Linker { status: ExitStatus, err: String },
    #[error("loader error: {0}")]
    Loader(libloading::Error),
    #[error("function raised status: {0}")]
    StatusRaised(String),
    #[error("encode error: {0}")]
    EncodeError(Box<dyn StdError + Send>),
    #[error("wrong layout: expected {expected}, got {got}")]
    WrongLayout {
        expected: layout::Layout,
        got: layout::Layout,
    },
    #[error("bad value: expected layout {expected}, got value {got}")]
    BadValue {
        expected: layout::Layout,
        got: layout::RefValue,
    },
    #[error("bincode error: {0}")]
    Bincode(bincode::Error),
    #[error("json error: {0}")]
    Json(serde_json::Error),
    #[error("zip error: {0}")]
    Zip(zip::result::ZipError),
    #[error("{0}")]
    Other(String),
    #[error("{error}\n\n{context}")]
    WithContext { error: Box<Error>, context: Ctx },
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<libloading::Error> for Error {
    fn from(err: libloading::Error) -> Error {
        Error::Loader(err)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(err: zip::result::ZipError) -> Error {
        Error::Zip(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Other(err)
    }
}

pub trait Context: Sized {
    fn with_context<F>(self, ctx: F) -> Self
    where
        F: FnOnce() -> String;

    fn context(self, ctx: &str) -> Self {
        self.with_context(|| ctx.to_string())
    }
}

#[derive(Debug, Default)]
pub struct Ctx(Vec<String>);

impl Display for Ctx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Context (most recent last):")?;
        for cause in &self.0 {
            writeln!(f, "    - {cause}")?;
        }

        Ok(())
    }
}

impl<T> Context for Result<T, Error> {
    fn with_context<F>(self, ctx: F) -> Self
    where
        F: FnOnce() -> String,
    {
        self.map_err(|err| {
            if let Error::WithContext { error, mut context } = err {
                context.0.push(ctx());
                Error::WithContext { error, context }
            } else {
                Error::WithContext {
                    error: Box::new(err),
                    context: Ctx(vec![ctx()]),
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::layout::{Layout, RefValue};
    use super::*;
    use byte_slice_cast::*;

    fn create_simple_graph() -> Graph {
        let mut graph = Graph::new();
        let RefValue::Scalar(a) = graph.input("a".to_string(), Layout::Scalar) else {
            unreachable!()
        };
        let RefValue::Scalar(b) = graph.input("b".to_string(), Layout::Scalar) else {
            unreachable!()
        };
        let c = graph.insert(op::Add, vec![a, b]).unwrap();
        let one = graph.r#const(1.0);
        let d = graph.insert(op::Add, vec![c, one]).unwrap();
        graph.output(RefValue::Scalar(d), Layout::Scalar).unwrap();

        graph
    }

    #[test]
    fn test_create_simple_graph() {
        create_simple_graph();
    }

    #[test]
    fn test_serialize_simple_graph() {
        let graph = create_simple_graph();
        println!("{}", serde_json::to_string_pretty(&graph).unwrap());
    }

    #[test]
    fn test_render_simple_graph() {
        let graph = create_simple_graph();
        println!("{}", graph.render());
    }

    #[test]
    fn test_assembly_simple_graph() {
        let graph = create_simple_graph();
        println!("{}", graph.render_assembly().unwrap());
    }

    #[test]
    fn test_compile_simple_graph() {
        let graph = create_simple_graph();
        graph.compile().unwrap();
    }

    #[test]
    fn test_run_simple_graph() {
        let graph = create_simple_graph();
        let func = graph.compile().unwrap();
        println!("{}", graph.render());
        println!("{}", graph.render_assembly().unwrap());

        let i = [5.0, 6.0];
        let out = func.eval_raw(i.as_byte_slice()).unwrap();
        println!("fn({:?}) = {:?}", i, out.as_slice_of::<f64>().unwrap());
    }

    fn create_pfunc_graph() -> Graph {
        let mut g = Graph::new();
        let RefValue::Scalar(a) = g.input("a".to_string(), Layout::Scalar) else {
            unreachable!()
        };
        let s = g.insert(op::Call("sqrt".to_string()), vec![a]).unwrap();
        g.output(RefValue::Scalar(s), Layout::Scalar).unwrap();

        g
    }

    #[test]
    fn test_pfunc_graph() {
        create_pfunc_graph();
    }

    #[test]
    fn test_run_pfunc() {
        let graph = create_pfunc_graph();
        let func = graph.compile().unwrap();
        println!("{}", graph.render());
        println!("{:?}", func);

        let num = 4.0;
        let sqrt: f64 = func
            .eval(&serde_json::to_value(format!("{{ \"a\": {num} }}")).unwrap())
            .unwrap();

        println!("sqrt({num}) = {sqrt}");
    }

    fn create_abs_graph() -> Graph {
        let mut g = Graph::new();
        let RefValue::Scalar(a) = g.input("a".to_string(), Layout::Scalar) else {
            unreachable!()
        };
        let aa = g.insert(op::Abs, vec![a]).unwrap();
        g.output(RefValue::Scalar(aa), Layout::Scalar).unwrap();

        g
    }

    #[test]
    fn test_abs_graph() {
        create_abs_graph();
    }

    #[test]
    fn test_run_abs() {
        let graph = create_abs_graph();
        let func = graph.compile().unwrap();
        println!("{}", graph.render());
        println!("{:?}", func);

        let num = 4.0;
        let abs: f64 = func
            .eval(&serde_json::to_value(format!("{{ \"a\": {num} }}")).unwrap())
            .unwrap();

        println!("abs({num}) = {abs}");

        let num = -4.0;
        let abs: f64 = func
            .eval(
                &serde_json::from_str::<serde_json::Value>(&format!("{{ \"a\": {num} }}")).unwrap(),
            )
            .unwrap();

        println!("abs({num}) = {abs}");
    }

    #[test]
    fn integration() {
        let func = Function::load(
            std::fs::File::open("../jyafn-go/pkg/jyafn/testdata/simple-ttl.jyafn").unwrap(),
        )
        .unwrap();

        let result: serde_json::Value = func.eval(&serde_json::from_str::<serde_json::Value>(
            "{\"virtual_provider_code\":\"BKX\",\"is_available\":false,\"day_distance\":1234}"
        ).unwrap()).unwrap();

        println!("{result:?}");
    }
}
