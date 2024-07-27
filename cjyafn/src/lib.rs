//! This crate exposes a minimal C-interface for Jyafn. The objective here is just to get
//! functions running anywhere (while also providing debugging info).

extern crate jyafn as rust;

#[cfg(test)]
mod test;

use get_size::GetSize;
use rust::{
    layout::{Layout, Struct},
    Error, Function, Graph,
};
use std::borrow::Cow;
use std::ffi::{c_char, CStr, CString};
use std::panic::UnwindSafe;
use std::sync::atomic::{AtomicIsize, Ordering};

/// Counts the number of strings allocated via `new_c_str` and freed through `free_str`.
/// This is meant for debugging, to detect leakages.
static N_ALLOCATED_STRS: AtomicIsize = AtomicIsize::new(0);

/// Every time this function is called, there needs to be an accompaning `free_str` on the
/// other side.
fn new_c_str(s: String) -> *const c_char {
    let c_str = CString::new(s)
        .unwrap_or_else(|err| {
            CString::new(String::from_utf8_lossy(&err.into_vec()).replace("\u{0}", ""))
                .expect("nulls have already been removed")
        })
        .into_boxed_c_str();
    N_ALLOCATED_STRS.fetch_add(1, Ordering::Relaxed);
    Box::leak(c_str) as *mut CStr as *const c_char
}

/// # Safety
///
/// Expects `s` to be a pointer _allocated_ C-style string. Only use this function with
/// pointers returned by functions in this library.
#[no_mangle]
pub unsafe extern "C" fn free_str(s: *const c_char) {
    let _c_str = Box::from_raw(s as *mut c_char);
    N_ALLOCATED_STRS.fetch_add(-1, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn n_allocated_strs() -> isize {
    N_ALLOCATED_STRS.load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn transmute_as_str(s: *mut ()) -> *mut c_char {
    s as *mut c_char
}

unsafe fn from_c_str<'a>(s: *const c_char) -> Cow<'a, str> {
    let cstr = CStr::from_ptr(s);
    cstr.to_string_lossy()
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Outcome(*mut ());

impl Outcome {
    fn from_result<T>(result: Result<T, Error>) -> Outcome {
        match result {
            Ok(ok) => {
                let boxed = Box::new(ok);
                let boxed_result = Box::new(Result::<*mut (), Error>::Ok(
                    Box::leak(boxed) as *mut T as *mut (),
                ));
                Outcome(Box::leak(boxed_result) as *mut Result<*mut (), Error> as *mut ())
            }
            Err(error) => {
                let boxed_result = Box::new(Result::<*mut (), Error>::Err(error));
                Outcome(Box::leak(boxed_result) as *mut Result<*mut (), Error> as *mut ())
            }
        }
    }
}

/// # Safety
///
/// Expects `outcome` to be a valid pointer to an outcome.
#[no_mangle]
pub unsafe extern "C" fn outcome_is_ok(outcome: Outcome) -> bool {
    let outcome = Box::from_raw(outcome.0 as *mut Result<*mut (), Error>);
    let is_ok = outcome.is_ok();
    Box::leak(outcome);
    is_ok
}

/// # Safety
///
/// Expects `outcome` to be a valid pointer to an outcome. The pointer becomes invalid
/// once it is passed to this function.
#[no_mangle]
pub unsafe extern "C" fn outcome_consume_ok(outcome: Outcome) -> *mut () {
    let outcome = Box::from_raw(outcome.0 as *mut Result<*mut (), Error>);
    outcome.expect("is supposed to be ok")
}

/// # Safety
///
/// Expects `outcome` to be a valid pointer to an outcome. The pointer becomes invalid
/// once it is passed to this function. Additionally, expects that the value of the "ok"
/// result is also a valid pointer, if this outcome is ok.
#[no_mangle]
pub unsafe extern "C" fn outcome_consume_ok_ptr(outcome: Outcome) -> *mut () {
    let outcome = Box::from_raw(outcome.0 as *mut Result<*mut (), Error>);
    let ok = outcome.expect("is supposed to be ok");
    let boxed_ptr = Box::from_raw(ok as *mut *mut ());
    *boxed_ptr
}

/// # Safety
///
/// Expects `outcome` to be a valid pointer to an outcome. The pointer becomes invalid
/// once it is passed to this function.
#[no_mangle]
pub unsafe extern "C" fn outcome_consume_err(outcome: Outcome) -> *const c_char {
    let outcome = Box::from_raw(outcome.0 as *mut Result<*mut (), Error>);
    let err = outcome.expect_err("is supposed to be err");
    new_c_str(err.to_string())
}

fn panic_to_outcome<F, T>(f: F) -> Outcome
where
    F: FnOnce() -> T + UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(outcome) => Outcome::from_result::<T>(Ok(outcome)),
        Err(_le_oops) => Outcome::from_result::<T>(Err(rust::Error::Other(
            "operation panicked (see stderr)".to_string(),
        ))),
    }
}

fn try_panic_to_outcome<F, T>(f: F) -> Outcome
where
    F: FnOnce() -> Result<T, Error> + UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(result) => Outcome::from_result::<T>(result),
        Err(_le_oops) => Outcome::from_result::<T>(Err(rust::Error::Other(
            "operation panicked (see stderr)".to_string(),
        ))),
    }
}

/// # Safety
///
/// Expects the `s` and the `fmt` parameters to point to valid C-style strings.
#[no_mangle]
pub unsafe extern "C" fn parse_datetime(s: *const c_char, fmt: *const c_char) -> Outcome {
    try_panic_to_outcome(|| {
        rust::utils::parse_datetime(&from_c_str(s), &from_c_str(fmt))
            .map(|dt| {
                Box::leak(Box::new(i64::from(rust::utils::Timestamp::from(dt)))) as *mut i64
                    as *const i64
            })
            .map_err(|e| e.to_string().into())
    })
}

/// # Safety
///
/// Expects `fmt` to be a valid pointer to a C-style string.
#[no_mangle]
pub unsafe extern "C" fn format_datetime(timestamp: i64, fmt: *const c_char) -> Outcome {
    panic_to_outcome(|| new_c_str(rust::utils::format_datetime(timestamp, &from_c_str(fmt))))
}

/// # Safety
///
/// Expects the `ptr` parameter to point to a valid _allocated_ `i64` in the heap. Only
/// use with pointers returned by functions in this library. This pointer becomes invalid
/// after it is passed to this function.
#[no_mangle]
pub unsafe extern "C" fn consume_i64_ptr(ptr: *mut i64) -> i64 {
    let boxed = Box::from_raw(ptr);
    *boxed
}

unsafe fn with_unchecked<T, U, F>(thing: *const (), f: F) -> U
where
    F: FnOnce(&T) -> U,
{
    let boxed = Box::from_raw(thing as *mut T);
    let outcome = f(&boxed);
    Box::leak(boxed);
    outcome
}

unsafe fn with<T, U, F>(thing: *const (), f: F) -> Outcome
where
    F: FnOnce(&T) -> U + UnwindSafe,
{
    panic_to_outcome(|| with_unchecked(thing, f))
}

unsafe fn try_with<T, U, F>(thing: *const (), f: F) -> Outcome
where
    F: FnOnce(&T) -> Result<U, Error> + UnwindSafe,
{
    try_panic_to_outcome(|| with_unchecked(thing, f))
}

#[allow(dead_code)]
unsafe fn with_mut_unchecked<T, U, F>(thing: *mut (), f: F) -> U
where
    F: FnOnce(&mut T) -> U,
{
    let mut boxed = Box::from_raw(thing as *mut T);
    let outcome = f(&mut boxed);
    Box::leak(boxed);
    outcome
}

#[allow(dead_code)]
unsafe fn with_mut<T, U, F>(thing: *mut (), f: F) -> Outcome
where
    F: FnOnce(&mut T) -> U + UnwindSafe,
{
    panic_to_outcome(|| with_mut_unchecked(thing, f))
}

#[allow(dead_code)]
unsafe fn try_with_mut<T, F>(thing: *mut (), f: F) -> Outcome
where
    F: FnOnce(&mut T) -> Result<T, Error> + UnwindSafe,
{
    try_panic_to_outcome(|| with_mut_unchecked(thing, f))
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph.
#[no_mangle]
pub unsafe extern "C" fn graph_name(graph: *const ()) -> *const c_char {
    with_unchecked(graph, |graph: &Graph| new_c_str(graph.name().to_string()))
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph and `key` to be a valid pointer
/// to a C-style string.
#[no_mangle]
pub unsafe extern "C" fn graph_get_metadata(graph: *const (), key: *const c_char) -> *const c_char {
    with_unchecked(graph, |graph: &Graph| {
        if let Some(value) = graph.metadata().get(&*from_c_str(key)) {
            new_c_str(value.to_string())
        } else {
            std::ptr::null()
        }
    })
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph.
#[no_mangle]
pub unsafe extern "C" fn graph_get_metadata_json(graph: *const ()) -> *const c_char {
    with_unchecked(graph, |graph: &Graph| {
        new_c_str(serde_json::to_string(graph.metadata()).expect("can always serialize json value"))
    })
}

/// # Safety
///
/// Expects `bytes` to point to the beginning of a valid byte slice in memory with the size
/// of _at least_ `len`.
#[no_mangle]
pub unsafe extern "C" fn graph_load(bytes: *const u8, len: usize) -> Outcome {
    try_panic_to_outcome(|| {
        Graph::load(std::io::Cursor::new(std::slice::from_raw_parts(bytes, len)))
    })
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph.
#[no_mangle]
pub unsafe extern "C" fn graph_to_json(graph: *const ()) -> *const c_char {
    with_unchecked(graph, |graph: &Graph| new_c_str(graph.to_json()))
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph.
#[no_mangle]
pub unsafe extern "C" fn graph_render(graph: *const ()) -> Outcome {
    with(graph, |graph: &Graph| new_c_str(graph.render().to_string()))
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph.
#[no_mangle]
pub unsafe extern "C" fn graph_compile(graph: *const ()) -> Outcome {
    try_with(graph, |graph: &Graph| graph.compile())
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph.
#[no_mangle]
pub unsafe extern "C" fn graph_clone(graph: *const ()) -> *const () {
    with_unchecked(graph, |graph: &Graph| {
        let boxed = Box::new(graph.clone());
        Box::leak(boxed) as *const Graph as *const ()
    })
}

/// # Safety
///
/// Expects `graph` to be a valid pointer to a graph. The pointer becomes invalid
/// after it is passed to this function.
#[no_mangle]
pub unsafe extern "C" fn graph_drop(graph: *mut ()) {
    let _ = Box::from_raw(graph as *mut Graph);
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_to_string(layout: *const ()) -> *const c_char {
    with_unchecked(layout, |layout: &Layout| new_c_str(layout.to_string()))
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_to_json(layout: *const ()) -> *const c_char {
    with_unchecked(layout, |layout: &Layout| {
        new_c_str(serde_json::to_string(layout).expect("can always serialize"))
    })
}

/// # Safety
///
/// Expects the `json` parameter to be a valid C-style string.
#[no_mangle]
pub unsafe extern "C" fn layout_from_json(json: *const c_char) -> Outcome {
    let decode = || -> Result<Layout, Error> {
        Ok(serde_json::Deserializer::from_str(&from_c_str(json))
            .into_iter::<Layout>()
            .next()
            .ok_or_else(|| "empty string".to_string())?
            .map_err(|err| err.to_string())?)
    };

    Outcome::from_result(decode())
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_size(layout: *const ()) -> usize {
    with_unchecked(layout, |layout: &Layout| layout.size())
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_is_unit(layout: *const ()) -> bool {
    with_unchecked(layout, |layout: &Layout| matches!(layout, Layout::Unit))
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_is_scalar(layout: *const ()) -> bool {
    with_unchecked(layout, |layout: &Layout| matches!(layout, Layout::Scalar))
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_is_bool(layout: *const ()) -> bool {
    with_unchecked(layout, |layout: &Layout| matches!(layout, Layout::Bool))
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.s
#[no_mangle]
pub unsafe extern "C" fn layout_is_datetime(layout: *const ()) -> bool {
    with_unchecked(layout, |layout: &Layout| {
        matches!(layout, Layout::DateTime(_))
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_is_symbol(layout: *const ()) -> bool {
    with_unchecked(layout, |layout: &Layout| matches!(layout, Layout::Symbol))
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_is_struct(layout: *const ()) -> bool {
    with_unchecked(layout, |layout: &Layout| {
        matches!(layout, Layout::Struct(_))
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_is_list(layout: *const ()) -> bool {
    with_unchecked(layout, |layout: &Layout| {
        matches!(layout, Layout::List(_, _))
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_datetime_format(layout: *const ()) -> *const c_char {
    with_unchecked(layout, |layout: &Layout| {
        if let Layout::DateTime(fmt) = layout {
            new_c_str(fmt.clone())
        } else {
            std::ptr::null()
        }
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_as_struct(layout: *const ()) -> *const () {
    with_unchecked(layout, |layout: &Layout| {
        if let Layout::Struct(s) = layout {
            s as *const Struct as *const ()
        } else {
            std::ptr::null()
        }
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_list_element(layout: *const ()) -> *const () {
    with_unchecked(layout, |layout: &Layout| {
        if let Layout::List(el, _) = layout {
            el.as_ref() as *const Layout as *const ()
        } else {
            std::ptr::null()
        }
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_list_size(layout: *const ()) -> usize {
    with_unchecked(layout, |layout: &Layout| {
        if let &Layout::List(_, size) = layout {
            size
        } else {
            0
        }
    })
}

/// # Safety
///
/// Expects the `layout` and the `other` parameters to be valid pointers to layouts.
#[no_mangle]
pub unsafe extern "C" fn layout_is_superset(layout: *mut (), other: *mut ()) -> bool {
    with_unchecked(layout, |layout: &Layout| {
        with_unchecked(other, |other: &Layout| layout.is_superset(other))
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout.
#[no_mangle]
pub unsafe extern "C" fn layout_clone(layout: *mut ()) -> *mut Layout {
    with_unchecked(layout, |layout: &Layout| {
        let boxed = Box::new(layout.clone());
        Box::leak(boxed)
    })
}

/// # Safety
///
/// Expects the `layout` parameter to be a valid pointer to a layout. The pointer becomes
/// invalid after it is passed to this function.
#[no_mangle]
pub unsafe extern "C" fn layout_drop(layout: *mut ()) {
    let _ = Box::from_raw(layout as *mut Layout);
}

/// # Safety
///
/// Expects the `strct` parameter to be a valid pointer to a jyafn struct.
#[no_mangle]
pub unsafe extern "C" fn strct_size(strct: *const ()) -> usize {
    with_unchecked(strct, |strct: &Struct| strct.0.len())
}

/// # Safety
///
/// Expects the `strct` parameter to be a valid pointer to a jyafn struct.
#[no_mangle]
pub unsafe extern "C" fn strct_get_item_name(strct: *const (), index: usize) -> *const c_char {
    with_unchecked(strct, |strct: &Struct| {
        // Remember, cannot panic, ever!
        if index < strct.0.len() {
            new_c_str(strct.0[index].0.clone())
        } else {
            std::ptr::null()
        }
    })
}

/// # Safety
///
/// Expects the `strct` parameter to be a valid pointer to a jyafn struct.
#[no_mangle]
pub unsafe extern "C" fn strct_get_item_layout(strct: *const (), index: usize) -> *const () {
    with_unchecked(strct, |strct: &Struct| {
        // Remember, cannot panic, ever!
        if index < strct.0.len() {
            &strct.0[index].1 as *const Layout as *const ()
        } else {
            std::ptr::null()
        }
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_name(func: *const ()) -> *const c_char {
    with_unchecked(func, |func: &Function| {
        new_c_str(func.graph().name().to_string())
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_input_size(func: *const ()) -> usize {
    with_unchecked(func, |func: &Function| func.input_size())
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_output_size(func: *const ()) -> usize {
    with_unchecked(func, |func: &Function| func.output_size())
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_input_layout(func: *const ()) -> *const () {
    with_unchecked(func, |func: &Function| {
        func.input_layout() as *const Layout as *const ()
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_output_layout(func: *const ()) -> *const () {
    with_unchecked(func, |func: &Function| {
        func.output_layout() as *const Layout as *const ()
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_graph(func: *const ()) -> *const () {
    with_unchecked(func, |func: &Function| {
        func.graph() as *const Graph as *const ()
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function and `key` to
/// be a pointer to a C-style string.
#[no_mangle]
pub unsafe extern "C" fn function_get_metadata(
    func: *const (),
    key: *const c_char,
) -> *const c_char {
    with_unchecked(func, |func: &Function| {
        if let Some(value) = func.graph().metadata().get(&*from_c_str(key)) {
            new_c_str(value.to_string())
        } else {
            std::ptr::null()
        }
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_get_metadata_json(func: *const ()) -> *const c_char {
    with_unchecked(func, |func: &Function| {
        new_c_str(
            serde_json::to_string(func.graph().metadata())
                .expect("can always serialize json value"),
        )
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_symbols_json(func: *const ()) -> *const c_char {
    with_unchecked(func, |func: &Function| {
        new_c_str(serde_json::to_string(func.graph().symbols()).expect("can always serialize"))
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_fn_ptr(
    func: *const (),
) -> unsafe extern "C" fn(*const u8, *mut u8) -> *mut rust::FnError {
    with_unchecked(func, |func: &Function| func.fn_ptr())
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function.
#[no_mangle]
pub unsafe extern "C" fn function_get_size(func: *const ()) -> usize {
    with_unchecked(func, |func: &Function| func.get_size())
}

/// # Safety
///
/// Expects `bytes` to point to the beginning of a valid byte slice in memory with the size
/// of _at least_ `len`.
#[no_mangle]
pub unsafe extern "C" fn function_load(bytes: *const u8, len: usize) -> Outcome {
    try_panic_to_outcome(|| {
        Function::load(std::io::Cursor::new(std::slice::from_raw_parts(bytes, len)))
    })
}

/// # Safety
///
/// Expects
/// 1. the `func` parameter to be a valid pointer to a jyafn function
/// 2. the `input` paramenter to be a valid pointer to a slice of size _at least_ the
///    function input size (given by `function_input_size`).
/// 3. the `output` paramenter to be a valid pointer to a slice of size _at least_ the
///    function output size (given by `function_output_size`).
#[no_mangle]
pub unsafe extern "C" fn function_call_raw(
    func: *const (),
    input: *const u8,
    output: *mut u8,
) -> Outcome {
    with_unchecked(func, |func: &Function| {
        match std::panic::catch_unwind(|| {
            let input = std::slice::from_raw_parts(input, func.input_size());
            let output = std::slice::from_raw_parts_mut(output, func.output_size());

            let fn_err = func.call_raw(input, output);
            if !fn_err.is_null() {
                let fn_err = Box::from_raw(fn_err).take();
                return Outcome::from_result(Result::<(), Error>::Err(fn_err.into()));
            }

            Outcome::from_result(Result::<(), Error>::Ok(()))
        }) {
            Ok(status) => status,
            Err(_le_oops) => Outcome::from_result(Result::<(), Error>::Err(
                "function raw call panicked (see stderr)".to_string().into(),
            )),
        }
    })
}

/// # Safety
///
/// Expects
/// 1. the `func` parameter to be a valid pointer to a jyafn function
/// 2. the `input` paramenter to be a valid pointer to a slice of size _at least_ the
///    function input size (given by `function_input_size`).
#[no_mangle]
pub unsafe extern "C" fn function_eval_raw(func: *const (), input: *const u8) -> Outcome {
    with(func, |func: &Function| {
        let input = std::slice::from_raw_parts(input, func.input_size());
        Outcome::from_result(
            func.eval_raw(input)
                .map(|output| Box::leak(output) as *const [u8] as *const ()),
        )
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function and `input` to
/// be a pointer to a C-style string.
#[no_mangle]
pub unsafe extern "C" fn function_eval_json(func: *const (), input: *mut c_char) -> Outcome {
    try_with(func, |func: &Function| {
        let input_cstr = CStr::from_ptr(input);
        let input_str = input_cstr.to_string_lossy();
        let input_value: serde_json::Value =
            serde_json::from_str(input_str.trim()).map_err(|e| e.to_string())?;
        let output_value: serde_json::Value = func.eval(&input_value)?;
        let output_str = serde_json::to_string(&output_value).expect("can serialize");
        let output_cstr = new_c_str(output_str);

        Ok(output_cstr)
    })
}

/// # Safety
///
/// Expects the `func` parameter to be a valid pointer to a jyafn function. The pointer
/// becomes invalid after it is passed to this function.
#[no_mangle]
pub unsafe extern "C" fn function_drop(func: *mut ()) {
    let _ = Box::from_raw(func as *mut Function);
}

// #[no_mangle]
// pub extern "C" fn pfunc_inscribe(
//     name: *const c_char,
//     fn_ptr: *const (),
//     signature: *const u8,
//     signature_len: usize,
//     returns: u8,
// ) -> Outcome {
//     unsafe {
//         try_panic_to_outcome(|| {
//             from_ptr_result((|| {
//                 let name_cstr = CStr::from_ptr(name);
//                 let name_str = name_cstr.to_string_lossy();
//                 let signature = std::slice::from_raw_parts(signature, signature_len)
//                     .iter()
//                     .copied()
//                     .map(rust::Type::try_from)
//                     .collect::<Result<Vec<_>, _>>()?;
//                 let returns: rust::Type = returns.try_into()?;

//                 rust::pfunc::inscribe(&name_str, fn_ptr, &signature, returns)
//                     .map(|_| std::ptr::null::<()>())
//             })())
//         })
//     }
// }
