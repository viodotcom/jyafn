use std::{cell::RefCell, fmt::Debug, sync::Arc};
use thread_local::ThreadLocal;

use super::{layout, Error, Graph, Type};

pub type RawFn = unsafe extern "C" fn(*const u8, *mut u8) -> u64;

#[derive(Debug)]
pub struct FunctionData {
    graph: Graph,
    _code: memmap::Mmap,
    input_layout: layout::Layout,
    output_layout: layout::Layout,
    input_size: usize,
    output_size: usize,
    fn_ptr: RawFn,
    input: ThreadLocal<RefCell<layout::Visitor>>,
    output: ThreadLocal<RefCell<layout::Visitor>>,
}

#[derive(Debug, Clone)]
pub struct Function {
    data: Arc<FunctionData>,
}

impl From<Arc<FunctionData>> for Function {
    fn from(data: Arc<FunctionData>) -> Function {
        Function { data }
    }
}

impl<'a> From<&'a Function> for Arc<FunctionData> {
    fn from(func: &'a Function) -> Arc<FunctionData> {
        func.data.clone()
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

    pub fn fn_ptr(&self) -> RawFn {
        self.data.fn_ptr
    }

    pub fn as_data(&self) -> Arc<FunctionData> {
        self.into()
    }

    pub fn load(bytes: &[u8]) -> Result<Function, Error> {
        let graph = Graph::load(bytes)?;
        graph.compile()
    }

    pub(crate) fn init(graph: Graph, shared_object: Vec<u8>) -> Result<Function, Error> {
        use object::{Object, ObjectSection, ObjectSymbol};
        let mut mmap = memmap::MmapMut::map_anon(shared_object.len())?;
        mmap.clone_from_slice(&shared_object);
        let code = mmap.make_exec()?;

        let input_layout = graph.input_layout.clone();
        let output_layout = graph.output_layout.clone();
        let obj = object::read::File::parse(code.as_ref())?;
        let symbol = obj.symbol_by_name("_run").unwrap();
        let section = obj.section_by_index(symbol.section_index().unwrap())?;
        let data = section.data()?;
        let fn_ptr: RawFn = unsafe { std::mem::transmute(data.as_ptr()) };
        let input_size_in_floats = input_layout.size();
        let output_size_in_floats = output_layout.size();

        Ok(Function {
            data: Arc::new(FunctionData {
                _code: code,
                input_size: input_size_in_floats * Type::Float.size(),
                input_layout: input_layout.into(),
                output_size: output_size_in_floats * Type::Float.size(),
                output_layout,
                fn_ptr,
                graph,
                input: ThreadLocal::new(),
                output: ThreadLocal::new(),
            }),
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
        } else if let Some(error) = self.graph().errors.get((status - 1) as usize) {
            Err(Error::StatusRaised(error.to_string()))
        } else {
            Err(Error::StatusRaised(format!(
                "unknown error of id {}",
                status - 1
            )))
        }
    }

    pub fn eval_with_decoder<E, D>(&self, input: &E, mut decoder: D) -> Result<D::Target, Error>
    where
        E: ?Sized + layout::Encode,
        D: layout::Decoder,
    {
        // Access buffers:
        let local_input = self
            .data
            .input
            .get_or(|| RefCell::new(layout::Visitor::new(self.data.input_size / 8)));
        let local_output = self
            .data
            .output
            .get_or(|| RefCell::new(layout::Visitor::new(self.data.output_size / 8)));
        let mut encode_visitor = local_input.borrow_mut();
        encode_visitor.reset();
        let mut decode_visitor = local_output.borrow_mut();
        decode_visitor.reset();

        // Define a symbols view (to store symbols present in the input not present in the
        // graph)
        let mut symbols_view = layout::SymbolsView::new(&self.data.graph.symbols);

        // Serialization dance:
        input
            .visit(
                &self.data.input_layout,
                &mut symbols_view,
                &mut encode_visitor,
            )
            .map_err(|err| Error::EncodeError(Box::new(err)))?;

        // Call:
        let status = self.call_raw(&encode_visitor.0, &mut decode_visitor.0);
        if status != 0 {
            return if let Some(error) = self.graph().errors.get((status - 1) as usize) {
                Err(Error::StatusRaised(error.to_string()))
            } else {
                Err(Error::StatusRaised(format!(
                    "unknown error of id {}",
                    status - 1
                )))
            };
        }

        // Deserialization dance:
        Ok(decoder.build(&self.data.output_layout, &symbols_view, &mut decode_visitor))
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
