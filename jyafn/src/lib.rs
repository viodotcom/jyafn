#[cfg(not(target_pointer_width = "64"))]
compile_error!("Currently `jyafn` only works in 64-bit atchitectures");

pub mod r#const;
pub mod dataset;
pub mod layout;
pub mod mapping;
pub mod op;
pub mod pfunc;

mod compile;
mod function;
mod graph;

#[cfg(feature = "map-reduce")]
pub use dataset::Dataset;
pub use function::{Function, FunctionData, RawFn};
pub use graph::{Graph, Node, Ref, Type};
pub use op::Op;
pub use r#const::Const;

use std::{error::Error as StdError, fmt::Debug, process::ExitStatus};

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
    Loader(object::Error),
    #[error("function raised status: {0}")]
    StatusRaised(String),
    #[error("encode error: {0}")]
    EncodeError(Box<dyn StdError + Send>),
    #[error("wrong layout: expected {expected:?}, got {got:?}")]
    WrongLayout {
        expected: layout::Layout,
        got: layout::Layout,
    },
    #[error("bad value: expected layout {expected:?}, got value {got:?}")]
    BadValue {
        expected: layout::Layout,
        got: layout::RefValue,
    },
    #[error("deserialization error: {0}")]
    Deserialization(bincode::Error),
    #[error("JSON deserialization error: {0}")]
    JsonDeserialization(serde_json::Error),
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<object::Error> for Error {
    fn from(err: object::Error) -> Error {
        Error::Loader(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Other(err)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use byte_slice_cast::*;

    fn create_simple_graph() -> Graph {
        let mut graph = Graph::new();
        let a = graph.scalar_input("a".to_string());
        let b = graph.scalar_input("b".to_string());
        let c = graph.insert(op::Add, vec![a, b]).unwrap();
        let one = graph.r#const(1.0);
        let d = graph.insert(op::Add, vec![c, one]).unwrap();
        graph.scalar_output(d);

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

        let i = [5.0, 6.0];
        let out = func.eval_raw(i.as_byte_slice()).unwrap();
        println!("fn({:?}) = {:?}", i, out.as_slice_of::<f64>().unwrap());
    }

    fn create_pfunc_graph() -> Graph {
        let mut g = Graph::new();
        let a = g.scalar_input("a".to_string());
        let s = g.insert(op::Call("sqrt".to_string()), vec![a]).unwrap();
        g.scalar_output(s);

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
            .eval(&layout::Value::Struct(maplit::hashmap! {
                "a".to_string() => layout::Value::Scalar(num),
            }))
            .unwrap();

        println!("sqrt({num}) = {sqrt}");
    }

    fn create_abs_graph() -> Graph {
        let mut g = Graph::new();
        let a = g.scalar_input("a".to_string());
        let aa = g.insert(op::Abs, vec![a]).unwrap();
        g.scalar_output(aa);

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
            .eval(&layout::Value::Struct(maplit::hashmap! {
                "a".to_string() => layout::Value::Scalar(num),
            }))
            .unwrap();

        println!("abs({num}) = {abs}");

        let num = -4.0;
        let abs: f64 = func
            .eval(&layout::Value::Struct(maplit::hashmap! {
                "a".to_string() => layout::Value::Scalar(num),
            }))
            .unwrap();

        println!("abs({num}) = {abs}");
    }
}
