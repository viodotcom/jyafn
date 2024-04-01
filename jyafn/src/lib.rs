pub mod r#const;
pub mod dataset;
pub mod layout;
pub mod op;
pub mod pfunc;

pub use dataset::Dataset;
pub use op::Op;
pub use r#const::Const;

use serde_derive::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    fmt::Debug,
    io::Write,
    process::{Command, ExitStatus, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
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
    #[error("qbe failed with {status}: {err}")]
    Qbe { status: ExitStatus, err: String },
    #[error("assembler failed with status {status}: {err}")]
    Assembler { status: ExitStatus, err: String },
    #[error("linker failed with status {status}: {err}")]
    Linker { status: ExitStatus, err: String },
    #[error("loader error: {0}")]
    Loader(object::Error),
    #[error("unction raised status {0}")]
    StatusRaised(u64),
    #[error("encode error")]
    EncodeError,
    #[error("wrong layout: expected {expected:?}, got {got:?}")]
    WrongLayout {
        expected: layout::Layout,
        got: layout::Layout,
    },
    #[error("deserialization error: {0}")]
    Deserialization(bincode::Error),
    #[error("JSON deserialization error: {0}")]
    JsonDeserialization(serde_json::Error),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    Float,
    Bool,
}

impl Type {
    fn render(self) -> qbe::Type<'static> {
        match self {
            Type::Float => qbe::Type::Double,
            Type::Bool => qbe::Type::Long,
        }
    }

    fn size(self) -> usize {
        match self {
            Type::Float => 8,
            Type::Bool => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Ref {
    Input(usize),
    Const(Type, u64),
    Node(usize),
}

impl Ref {
    fn render(self) -> qbe::Value {
        match self {
            Ref::Input(input_id) => qbe::Value::Temporary(format!("i{input_id}")),
            Ref::Const(_, r#const) => qbe::Value::Const(r#const),
            Ref::Node(node_id) => qbe::Value::Temporary(format!("n{node_id}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Input {
    ty: Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    op: Arc<dyn Op>,
    args: Vec<Ref>,
    ty: Type,
}

impl Node {
    pub fn init<O: Op>(graph: &Graph, op: O, args: Vec<Ref>) -> Result<Node, Error> {
        let arg_types = args.iter().map(|r| graph.type_of(*r)).collect::<Vec<_>>();
        let Some(ty) = op.annotate(&arg_types) else {
            return Err(Error::Type(Box::new(op), arg_types));
        };

        Ok(Node {
            op: Arc::new(op),
            args,
            ty,
        })
    }
}

const GRAPH_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    name: String,
    input_layout: layout::Struct,
    output_layout: layout::Layout,
    nodes: Vec<Node>,
    inputs: Vec<Input>,
    outputs: Vec<Ref>,
}

impl Graph {
    pub fn new_with_name(name: String) -> Graph {
        Graph {
            name,
            input_layout: layout::Struct::default(),
            output_layout: layout::Layout::Unit,
            nodes: vec![],
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn new() -> Graph {
        let graph_id = GRAPH_ID.fetch_add(1, Ordering::Relaxed);
        Graph::new_with_name(format!("g{graph_id}"))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn dump(&self) -> Vec<u8> {
        bincode::serialize(self).expect("can always serialize")
    }

    pub fn load(bytes: &[u8]) -> Result<Self, Error> {
        bincode::deserialize(bytes).map_err(Error::Deserialization)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("can always serialize")
    }

    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::JsonDeserialization)
    }

    fn type_of(&self, reference: Ref) -> Type {
        match reference {
            Ref::Node(node_id) => self.nodes[node_id].ty,
            Ref::Input(input_id) => self.inputs[input_id].ty,
            Ref::Const(ty, _) => ty,
        }
    }

    pub fn r#const<C: Const>(&mut self, r#const: C) -> Ref {
        Ref::Const(r#const.annotate(), r#const.render().into())
    }

    pub fn insert<O: Op>(&mut self, op: O, args: Vec<Ref>) -> Result<Ref, Error> {
        let current_id = self.nodes.len();
        self.nodes.push(Node::init(&self, op, args)?);

        Ok(Ref::Node(current_id))
    }

    fn push_input(&mut self, ty: Type) -> Ref {
        let current_id = self.inputs.len();
        self.inputs.push(Input { ty });

        Ref::Input(current_id)
    }

    pub fn input(&mut self, name: String) -> Ref {
        self.input_layout.insert(name, layout::Layout::Scalar);
        self.push_input(Type::Float)
    }

    pub fn vec_input(&mut self, name: String, size: usize) -> Vec<Ref> {
        self.input_layout.insert(
            name,
            layout::Layout::List(Box::new(layout::Layout::Scalar), size),
        );
        (0..size).map(|_| self.push_input(Type::Float)).collect()
    }

    pub fn enum_input(&mut self, name: String, options: Vec<String>) -> Ref {
        self.input_layout
            .insert(name, layout::Layout::Enum(options));
        self.push_input(Type::Float)
    }

    pub fn output(&mut self, scalar: Ref) {
        self.outputs = vec![scalar];
        self.output_layout = layout::Layout::Scalar;
    }

    pub fn slice_output(&mut self, slice: &[Ref]) {
        self.outputs = slice.to_vec();
        self.output_layout = layout::Layout::List(Box::new(layout::Layout::Scalar), slice.len());
    }

    pub fn render(&self) -> qbe::Module {
        let mut module = qbe::Module::new();
        let main = module.add_function(qbe::Function::new(
            qbe::Linkage::public(),
            "run".to_string(),
            vec![
                (qbe::Type::Long, qbe::Value::Temporary("in".to_string())),
                (qbe::Type::Long, qbe::Value::Temporary("out".to_string())),
            ],
            Some(qbe::Type::Long),
        ));
        main.add_block("start".to_string());

        for (id, input) in self.inputs.iter().enumerate() {
            main.assign_instr(
                qbe::Value::Temporary(format!("i{id}")),
                input.ty.render(),
                qbe::Instr::Load(input.ty.render(), qbe::Value::Temporary("in".to_string())),
            );
            main.assign_instr(
                qbe::Value::Temporary("in".to_string()),
                qbe::Type::Long,
                qbe::Instr::Add(
                    qbe::Value::Const(input.ty.size() as u64),
                    qbe::Value::Temporary("in".to_string()),
                ),
            );
        }

        // Supposes that the nodes were already declared in topological order:
        for (id, node) in self.nodes.iter().enumerate() {
            node.op
                .render_into(Ref::Node(id).render(), &node.args, main)
        }

        for output in &self.outputs {
            main.add_instr(qbe::Instr::Store(
                self.type_of(*output).render(),
                qbe::Value::Temporary("out".to_string()),
                output.render(),
            ));
            main.assign_instr(
                qbe::Value::Temporary("out".to_string()),
                qbe::Type::Long,
                qbe::Instr::Add(
                    qbe::Value::Const(self.type_of(*output).size() as u64),
                    qbe::Value::Temporary("out".to_string()),
                ),
            );
        }

        main.add_instr(qbe::Instr::Ret(Some(qbe::Value::Const(0))));

        module
    }

    // fn render(&self) -> &'static str {
    //     r#"
    //     export function l $run(l %in, l %out) {
    //         @start
    //                 %i0 =d loadd %in
    //                 %in =l add 8, %in
    //                 %i1 =d loadd %in
    //                 %in =l add 8, %in
    //                 %n0 =d add %i0, %i1
    //                 %c0 =d loadd $c0
    //                 %n1 =d add %n0, %c0
    //                 stored %n1, %out
    //                 %out =l add 8, %out
    //                 ret 0
    //         }
    //         data $c0 = { d d_1 }
    //     "#
    // }

    pub fn render_assembly(&self) -> Result<String, Error> {
        let rendered = self.render();
        Ok(create_assembly(rendered)?)
    }

    pub fn compile(&self) -> Result<Function, Error> {
        let rendered = self.render();
        let assembly = create_assembly(rendered)?;
        let unlinked = assemble(&assembly)?;
        let shared_object = link(&unlinked)?;

        Function::init(
            Graph::clone(self),
            self.input_layout.clone(),
            self.output_layout.clone(),
            shared_object,
        )
    }
}

fn create_assembly<R>(rendered: R) -> Result<String, Error>
where
    R: std::fmt::Display,
{
    let mut qbe = Command::new("qbe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = qbe.stdin.take().expect("qbe stdin stream not captured");
    stdin.write_all(rendered.to_string().as_bytes())?;
    drop(stdin);

    let qbe_output = qbe.wait_with_output()?;
    if !qbe_output.status.success() {
        return Err(Error::Qbe {
            status: qbe_output.status,
            err: String::from_utf8_lossy(&qbe_output.stderr).to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&qbe_output.stdout).to_string())
}

fn assemble(assembly: &str) -> Result<Vec<u8>, Error> {
    let mut r#as = Command::new("as")
        .args(["-o", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = r#as.stdin.take().expect("qbe stdin stream not captured");
    stdin.write_all(assembly.as_bytes())?;
    drop(stdin);

    let as_output = r#as.wait_with_output()?;
    if !as_output.status.success() {
        return Err(Error::Assembler {
            status: as_output.status,
            err: String::from_utf8_lossy(&as_output.stderr).to_string(),
        });
    }

    Ok(as_output.stdout)
}

fn link(unlinked: &[u8]) -> Result<Vec<u8>, Error> {
    let tempdir = tempfile::tempdir()?;
    let input = tempdir.path().join("main.o");
    let output = tempdir.path().join("main.so");
    std::fs::write(&input, unlinked)?;

    let linker = Command::new("gcc")
        .arg("-shared")
        .arg(input)
        .arg("-o")
        .arg(&output)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    if !linker.status.success() {
        return Err(Error::Linker {
            status: linker.status,
            err: String::from_utf8_lossy(&linker.stderr).to_string(),
        });
    }

    Ok(std::fs::read(output)?)
}

type RawFn = unsafe extern "C" fn(*const u8, *mut u8) -> u64;

#[derive(Debug)]
struct FunctionData {
    graph: Graph,
    _code: memmap::Mmap,
    input_layout: layout::Layout,
    output_layout: layout::Layout,
    input_size: usize,
    output_size: usize,
    fn_ptr: RawFn,
}

#[derive(Debug)]
pub struct Function {
    data: Arc<FunctionData>,
    input: RefCell<layout::Visitor>,
    output: RefCell<layout::Visitor>,
}

impl Clone for Function {
    fn clone(&self) -> Function {
        Function {
            data: self.data.clone(),
            input: RefCell::new(layout::Visitor::new_like(&*self.input.borrow())),
            output: RefCell::new(layout::Visitor::new_like(&*self.output.borrow())),
        }
    }
}

impl Function {
    pub fn input_size(&self) -> usize {
        self.data.input_size
    }

    pub fn output_size(&self) -> usize {
        self.data.output_size
    }

    pub fn input_layout(&self) -> &layout::Layout {
        &self.data.input_layout
    }

    pub fn output_layout(&self) -> &layout::Layout {
        &self.data.output_layout
    }

    pub fn graph(&self) -> &Graph {
        &self.data.graph
    }

    pub fn load(bytes: &[u8]) -> Result<Function, Error> {
        let graph = Graph::load(bytes)?;
        graph.compile()
    }

    fn init(
        graph: Graph,
        input_layout: layout::Struct,
        output_layout: layout::Layout,
        shared_object: Vec<u8>,
    ) -> Result<Function, Error> {
        use object::{Object, ObjectSection, ObjectSymbol};
        let mut mmap = memmap::MmapMut::map_anon(shared_object.len())?;
        mmap.clone_from_slice(&shared_object);
        let code = mmap.make_exec()?;

        let obj = object::read::File::parse(code.as_ref())?;
        let symbol = obj.symbol_by_name("_run").unwrap();
        let section = obj.section_by_index(symbol.section_index().unwrap())?;
        let data = section.data()?;
        let fn_ptr: RawFn = unsafe { std::mem::transmute(data.as_ptr()) };
        let input_size_in_floats = input_layout.size();
        let output_size_in_floats = output_layout.size();

        Ok(Function {
            data: Arc::new(FunctionData {
                graph,
                _code: code,
                input_size: input_size_in_floats * Type::Float.size(),
                input_layout: input_layout.into(),
                output_size: output_size_in_floats * Type::Float.size(),
                output_layout,
                fn_ptr,
            }),
            input: RefCell::new(layout::Visitor::new(input_size_in_floats)),
            output: RefCell::new(layout::Visitor::new(output_size_in_floats)),
        })
    }

    pub fn call_raw<I, O>(&self, input: I, mut output: O) -> u64
    where
        I: AsRef<[u8]>,
        O: AsMut<[u8]>,
    {
        let input = input.as_ref();
        let output = output.as_mut();

        assert_eq!(self.data.input_size, input.len());
        assert_eq!(self.data.output_size, output.len());

        // Safety: input and output sizes are checked and function pinky-promisses not to
        // accesses anything out of bounds.
        unsafe { (self.data.fn_ptr)(input.as_ptr() as *const u8, output.as_mut_ptr() as *mut u8) }
    }

    pub fn eval_raw<I>(&self, input: I) -> Result<Box<[u8]>, Error>
    where
        I: AsRef<[u8]>,
    {
        let mut output = vec![0; self.data.output_size as usize].into_boxed_slice();
        let status = self.call_raw(input, &mut output);
        if status == 0 {
            Ok(output)
        } else {
            Err(Error::StatusRaised(status))
        }
    }

    pub fn eval_with_decoder<E, D>(&self, input: &E, mut decoder: D) -> Result<D::Target, Error>
    where
        E: ?Sized + layout::Encode,
        D: layout::Decoder,
    {
        // Access buffers:
        let mut encode_visitor = self.input.borrow_mut();
        encode_visitor.reset();
        let mut decode_visitor = self.output.borrow_mut();
        decode_visitor.reset();

        // Serialization dance:
        input
            .visit(&self.data.input_layout, &mut encode_visitor)
            .map_err(|_| Error::EncodeError)?;

        // Call:
        let status = self.call_raw(&encode_visitor.0, &mut decode_visitor.0);
        if status != 0 {
            return Err(Error::StatusRaised(status));
        }

        // Deserialization dance:
        Ok(decoder.build(&self.data.output_layout, &mut decode_visitor))
    }

    pub fn eval<E, D>(&self, input: &E) -> Result<D, Error>
    where
        E: ?Sized + layout::Encode,
        D: layout::Decode,
    {
        let zero = layout::ZeroDecoder::new();
        self.eval_with_decoder(input, zero)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use byte_slice_cast::*;

    fn create_simple_graph() -> Graph {
        let mut graph = Graph::new();
        let a = graph.input("a".to_string());
        let b = graph.input("b".to_string());
        let c = graph.insert(op::Add, vec![a, b]).unwrap();
        let one = graph.r#const(1.0);
        let d = graph.insert(op::Add, vec![c, one]).unwrap();
        graph.output(d);

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
        let a = g.input("a".to_string());
        let s = g.insert(op::Call("sqrt".to_string()), vec![a]).unwrap();
        g.output(s);

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
        let a = g.input("a".to_string());
        let aa = g.insert(op::Abs, vec![a]).unwrap();
        g.output(aa);

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
