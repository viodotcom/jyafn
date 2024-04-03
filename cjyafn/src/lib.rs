//! This crate exposes a minimal C-interface for Jyafn. The objective here is just to get
//! functions running anywhere (while also providing debugging info).

extern crate jyafn as rust;

use rust::{
    layout::{Layout, Struct, Visitor},
    Error, Function, Graph,
};
use std::ffi::{c_char, CStr, CString};

#[repr(C)]
pub struct Outcome {
    ok: *mut (),
    err: *const (),
}

fn from_result<T>(result: Result<T, Error>) -> Outcome {
    match result {
        Ok(ok) => {
            let boxed = Box::new(ok);
            Outcome {
                ok: Box::leak(boxed) as *mut T as *mut (),
                err: std::ptr::null(),
            }
        }
        Err(error) => {
            let boxed = Box::new(error);
            Outcome {
                ok: std::ptr::null_mut(),
                err: Box::leak(boxed) as *const Error as *const (),
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn error_to_string(err: *const()) -> *const c_char {
    unsafe {
        with(err, |err: &Error| {
            new_c_str(err.to_string())
        })
    }
}

fn from_ptr_result<T>(result: Result<*const T, Error>) -> Outcome {
    match result {
        Ok(ok) => Outcome {
            ok: ok as *mut (),
            err: std::ptr::null(),
        },
        Err(error) => {
            let boxed = Box::new(error);
            Outcome {
                ok: std::ptr::null_mut(),
                err: Box::leak(boxed) as *const Error as *const (),
            }
        }
    }
}

fn new_c_str(s: String) -> *const c_char {
    let c_str = CString::new(s)
        .expect("string representation should never contain \\0")
        .into_boxed_c_str();
    Box::leak(c_str) as *mut CStr as *const c_char
}

unsafe fn with<T, U, F>(thing: *const (), f: F) -> U
where
    F: FnOnce(&T) -> U,
{
    let boxed = Box::from_raw(thing as *mut T);
    let outcome = f(&boxed);
    Box::leak(boxed);
    outcome
}

unsafe fn with_mut<T, U, F>(thing: *mut (), f: F) -> U
where
    F: FnOnce(&mut T) -> U,
{
    let mut boxed = Box::from_raw(thing as *mut T);
    let outcome = f(&mut boxed);
    Box::leak(boxed);
    outcome
}

#[no_mangle]
pub extern "C" fn error_display(error: *const ()) -> *const c_char {
    unsafe { with(error, |error: &Error| new_c_str(error.to_string())) }
}

#[no_mangle]
pub extern "C" fn graph_load(bytes: *const u8, len: usize) -> Outcome {
    fn graph_load(bytes: *const u8, len: usize) -> Result<Graph, Error> {
        unsafe { Graph::load(std::slice::from_raw_parts(bytes, len)) }
    }

    from_result(graph_load(bytes, len))
}

#[no_mangle]
pub extern "C" fn graph_to_json(graph: *const ()) -> *const c_char {
    unsafe { with(graph, |graph: &Graph| new_c_str(graph.to_json())) }
}

#[no_mangle]
pub extern "C" fn graph_render(graph: *const ()) -> *const c_char {
    unsafe { with(graph, |graph: &Graph| new_c_str(graph.render().to_string())) }
}

#[no_mangle]
pub extern "C" fn graph_compile(graph: *const ()) -> Outcome {
    unsafe { with(graph, |graph: &Graph| from_result(graph.compile())) }
}

#[no_mangle]
pub extern "C" fn graph_clone(graph: *const ()) -> *const () {
    unsafe {
        with(graph, |graph: &Graph| {
            let boxed = Box::new(graph.clone());
            Box::leak(boxed) as *const Graph as *const ()
        })
    }
}

#[no_mangle]
pub extern "C" fn layout_to_json(layout: *const ()) -> *const c_char {
    unsafe {
        with(layout, |layout: &Layout| {
            new_c_str(serde_json::to_string(layout).expect("can always serialize"))
        })
    }
}

#[no_mangle]
pub extern "C" fn layout_size(layout: *const ()) -> usize {
    unsafe { with(layout, |layout: &Layout| layout.size()) }
}

#[no_mangle]
pub extern "C" fn layout_is_unit(layout: *const ()) -> bool {
    unsafe { with(layout, |layout: &Layout| matches!(layout, Layout::Unit)) }
}

#[no_mangle]
pub extern "C" fn layout_is_scalar(layout: *const ()) -> bool {
    unsafe { with(layout, |layout: &Layout| matches!(layout, Layout::Scalar)) }
}

#[no_mangle]
pub extern "C" fn layout_is_struct(layout: *const ()) -> bool {
    unsafe {
        with(layout, |layout: &Layout| {
            matches!(layout, Layout::Struct(_))
        })
    }
}

#[no_mangle]
pub extern "C" fn layout_is_enum(layout: *const ()) -> bool {
    unsafe { with(layout, |layout: &Layout| matches!(layout, Layout::Enum(_))) }
}

#[no_mangle]
pub extern "C" fn layout_is_list(layout: *const ()) -> bool {
    unsafe {
        with(layout, |layout: &Layout| {
            matches!(layout, Layout::List(_, _))
        })
    }
}

#[no_mangle]
pub extern "C" fn layout_as_struct(layout: *const ()) -> *const () {
    unsafe {
        with(layout, |layout: &Layout| {
            if let Layout::Struct(s) = layout {
                s as *const Struct as *const ()
            } else {
                std::ptr::null()
            }
        })
    }
}

#[no_mangle]
pub extern "C" fn layout_list_element(layout: *const ()) -> *const () {
    unsafe {
        with(layout, |layout: &Layout| {
            if let Layout::List(el, _) = layout {
                el.as_ref() as *const Layout as *const ()
            } else {
                std::ptr::null()
            }
        })
    }
}

#[no_mangle]
pub extern "C" fn layout_list_size(layout: *const ()) -> usize {
    unsafe {
        with(layout, |layout: &Layout| {
            if let &Layout::List(_, size) = layout {
                size
            } else {
                0
            }
        })
    }
}

#[no_mangle]
pub extern "C" fn strct_size(strct: *const ()) -> usize {
    unsafe { with(strct, |strct: &Struct| strct.0.len()) }
}

#[no_mangle]
pub extern "C" fn strct_get_item_name(strct: *const (), index: usize) -> *const c_char {
    unsafe {
        with(strct, |strct: &Struct| {
            // Remember, cannot panic, ever!
            if index < strct.0.len() {
                new_c_str(strct.0[index].0.clone())
            } else {
                std::ptr::null()
            }
        })
    }
}

#[no_mangle]
pub extern "C" fn strct_get_item_layout(strct: *const (), index: usize) -> *const () {
    unsafe {
        with(strct, |strct: &Struct| {
            // Remember, cannot panic, ever!
            if index < strct.0.len() {
                &strct.0[index].1 as *const Layout as *const ()
            } else {
                std::ptr::null()
            }
        })
    }
}

#[no_mangle]
pub extern "C" fn visitor_push(visitor: *mut (), val: f64) {
    unsafe { with_mut(visitor, |visitor: &mut Visitor| visitor.push(val)) }
}

#[no_mangle]
pub extern "C" fn visitor_pop(visitor: *mut ()) -> f64 {
    unsafe { with_mut(visitor, |visitor: &mut Visitor| visitor.pop()) }
}

#[no_mangle]
pub extern "C" fn function_input_size(func: *const ()) -> usize {
    unsafe { with(func, |func: &Function| func.input_size()) }
}

#[no_mangle]
pub extern "C" fn function_output_size(func: *const ()) -> usize {
    unsafe { with(func, |func: &Function| func.output_size()) }
}

#[no_mangle]
pub extern "C" fn function_input_layout(func: *const ()) -> *const () {
    unsafe {
        with(func, |func: &Function| {
            func.input_layout() as *const Layout as *const ()
        })
    }
}

#[no_mangle]
pub extern "C" fn function_output_layout(func: *const ()) -> *const () {
    unsafe {
        with(func, |func: &Function| {
            func.output_layout() as *const Layout as *const ()
        })
    }
}

#[no_mangle]
pub extern "C" fn function_graph(func: *const ()) -> *const () {
    unsafe {
        with(func, |func: &Function| {
            func.graph() as *const Graph as *const ()
        })
    }
}

#[no_mangle]
pub extern "C" fn function_fn_ptr(
    func: *const (),
) -> unsafe extern "C" fn(*const u8, *mut u8) -> u64 {
    unsafe { with(func, |func: &Function| func.fn_ptr()) }
}

#[no_mangle]
pub extern "C" fn function_load(bytes: *const u8, len: usize) -> Outcome {
    fn function_load(bytes: *const u8, len: usize) -> Result<Function, Error> {
        unsafe { Function::load(std::slice::from_raw_parts(bytes, len)) }
    }

    from_result(function_load(bytes, len))
}

#[no_mangle]
pub extern "C" fn function_call_raw(func: *const (), input: *const u8, output: *mut u8) -> u64 {
    unsafe {
        with(func, |func: &Function| {
            let input = std::slice::from_raw_parts(input, func.input_size());
            let output = std::slice::from_raw_parts_mut(output, func.output_size());
            func.call_raw(input, output)
        })
    }
}

#[no_mangle]
pub extern "C" fn function_eval_raw(func: *const (), input: *const u8) -> Outcome {
    unsafe {
        with(func, |func: &Function| {
            let input = std::slice::from_raw_parts(input, func.input_size());
            from_result(
                func.eval_raw(input)
                    .map(|output| Box::leak(output) as *const [u8] as *const ()),
            )
        })
    }
}

#[repr(C)]
pub struct ExternEncodable {
    data_ptr: *const (),
    encode: unsafe extern "C" fn(*const (), *const (), *mut ()) -> bool,
}

impl rust::layout::Encode for ExternEncodable {
    fn visit(&self, layout: &Layout, visitor: &mut Visitor) -> Result<(), ()> {
        let is_ok = unsafe {
            (self.encode)(
                self.data_ptr,
                layout as *const Layout as *const (),
                visitor as *mut Visitor as *mut (),
            )
        };

        if is_ok {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[repr(C)]
pub struct ExternDecoder {
    data_ptr: *mut (),
    decode: unsafe extern "C" fn(*mut (), *const (), *mut ()) -> *const (),
}

impl rust::layout::Decoder for ExternDecoder {
    type Target = *const ();
    fn build(&mut self, layout: &Layout, visitor: &mut Visitor) -> Self::Target {
        unsafe {
            (self.decode)(
                self.data_ptr,
                layout as *const Layout as *const (),
                visitor as *mut Visitor as *mut (),
            )
        }
    }
}

#[no_mangle]
pub extern "C" fn function_eval(
    func: *const (),
    input: ExternEncodable,
    decoder: ExternDecoder,
) -> Outcome {
    unsafe {
        with(func, |func: &Function| {
            from_ptr_result(func.eval_with_decoder(&input, decoder))
        })
    }
}

#[no_mangle]
pub extern "C" fn function_eval_json(func: *const (), input: *mut c_char) -> Outcome {
    unsafe {
        with(func, |func: &Function| {
            from_ptr_result((|| {
                let input_cstr = CStr::from_ptr(input);
                let input_str = input_cstr.to_string_lossy();
                let input_value: serde_json::Value =
                    serde_json::from_str(&*input_str).map_err(|e| e.to_string())?;
                let output_value: serde_json::Value = func.eval(&input_value)?;
                let output_str = serde_json::to_string(&output_value).expect("can serialize");
                Ok(new_c_str(output_str))
            })())
        })
    }
}
