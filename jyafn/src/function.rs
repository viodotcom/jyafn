use get_size::GetSize;
use object::Object;
use std::ffi::{c_char, CStr};
use std::{
    cell::RefCell,
    fmt::Debug,
    io::{Read, Seek},
    sync::Arc,
};
use thread_local::ThreadLocal;

use super::{layout, Error, Graph, Type};

pub type RawFn = unsafe extern "C" fn(*const u8, *mut u8) -> *const c_char;

#[derive(Debug)]
pub struct FunctionData {
    graph: Graph,
    code: memmap::Mmap,
    input_layout: layout::Layout,
    output_layout: layout::Layout,
    input_size: usize,
    output_size: usize,
    fn_ptr: RawFn,
    input: ThreadLocal<RefCell<layout::Visitor>>,
    output: ThreadLocal<RefCell<layout::Visitor>>,
}

impl GetSize for FunctionData {
    fn get_heap_size(&self) -> usize {
        self.graph.get_heap_size()
            + self.code.len()
            + self.input_layout.get_heap_size()
            + self.output_layout.get_heap_size()
            + self
                .input
                .get()
                .map(|i| i.borrow().as_ref().get_size())
                .unwrap_or(0)
            + self
                .output
                .get()
                .map(|o| o.borrow().as_ref().get_size())
                .unwrap_or(0)
    }
}

#[derive(Debug, Clone, GetSize)]
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

    pub fn load<R: Read + Seek>(reader: R) -> Result<Function, Error> {
        let graph = Graph::load(reader)?;
        graph.compile()
    }

    pub(crate) fn init(graph: Graph, shared_object: Vec<u8>) -> Result<Function, Error> {
        let mut mmap = memmap::MmapMut::map_anon(shared_object.len())?;
        mmap.clone_from_slice(&shared_object);
        let code = mmap.make_exec()?;

        let obj = object::read::File::parse(code.as_ref())?;

        #[cfg(target_os = "macos")]
        const ENTRYPOINT: &[u8] = b"_run";
        #[cfg(target_os = "linux")]
        const ENTRYPOINT: &[u8] = b"run";

        let exports = obj.exports()?;
        let entry = exports
            .into_iter()
            .find(|export| export.name() == ENTRYPOINT)
            .expect("entrypoint not found");
        let start_ptr = code.as_ptr().wrapping_add(entry.address() as usize);
        let fn_ptr: RawFn = unsafe { std::mem::transmute(start_ptr) };

        let input_layout = graph.input_layout.clone();
        let output_layout = graph.output_layout.clone();
        let input_size_in_floats = input_layout.size();
        let output_size_in_floats = output_layout.size();

        let mut data = FunctionData {
            code,
            input_size: input_size_in_floats * Type::Float.size(),
            input_layout: input_layout.into(),
            output_size: output_size_in_floats * Type::Float.size(),
            output_layout,
            fn_ptr,
            graph,
            input: ThreadLocal::new(),
            output: ThreadLocal::new(),
        };

        let data_size = data.get_size();
        data.graph
            .metadata_mut()
            .insert("jyafn.mem_size_estimate".to_string(), data_size.to_string());

        Ok(Function {
            data: Arc::new(data),
        })
    }

    pub fn call_raw<I, O>(&self, input: I, mut output: O) -> *const c_char
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
        unsafe { (self.data.fn_ptr)(input.as_ptr(), output.as_mut_ptr()) }
    }

    pub fn eval_raw<I>(&self, input: I) -> Result<Box<[u8]>, Error>
    where
        I: AsRef<[u8]>,
    {
        let mut output = vec![0; self.data.output_size].into_boxed_slice();
        let status = self.call_raw(input, &mut output);
        if status == std::ptr::null() {
            Ok(output)
        } else {
            // Safety: null was checked and the function pinky-promisses to return a valid
            // C string in case of error.
            let error = unsafe { CStr::from_ptr(status) };
            Err(Error::StatusRaised(error.to_string_lossy().to_string()))
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
        if status != std::ptr::null() {
            // Safety: null was checked and the function pinky-promisses to return a valid
            // C string in case of error.
            let error = unsafe { CStr::from_ptr(status) };
            return Err(Error::StatusRaised(error.to_string_lossy().to_string()));
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
