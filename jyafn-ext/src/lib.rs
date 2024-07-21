//! This crate is intended to help extension authors. It exposes a minimal version of
//! `jyafn` and many convenience macros to generate all the boilerplate involved.

mod fn_error;
mod io;
mod layout;
mod outcome;
mod resource;

/// Reexporting from the `paste` crate. This is neeede because we have to programatically
/// generate new identifiers to be used as symbols in the final shared object.
pub use paste::paste;
/// We need JSON support to zip JSON values around the FFI boundary.
pub use serde_json;

pub use fn_error::FnError;
pub use io::{Input, OutputBuilder};
pub use layout::{Layout, Struct, ISOFORMAT};
pub use outcome::Outcome;
pub use resource::{Method, Resource};

/// Generates the boilerplate code for a `jyafn` extension.
#[macro_export]
macro_rules! extension {
    ($($ty:ty),*) => {
        use std::ffi::{c_char, CString};
        use $crate::Outcome;

        #[no_mangle]
        pub extern "C" fn outcome_get_err(outcome: *mut Outcome) -> *const c_char {
            let outcome = unsafe {
                // Safety: expecting a valid pointer from input.
                &*outcome
            };

            match outcome {
                Outcome::Ok(_) => std::ptr::null(),
                Outcome::Err(err) => err.as_ptr(),
            }
        }

        #[no_mangle]
        pub extern "C" fn outcome_get_ok(outcome: *mut Outcome) -> *mut () {
            let outcome = unsafe {
                // Safety: expecting a valid pointer from input.
                &*outcome
            };

            match outcome {
                Outcome::Ok(ptr) => *ptr,
                Outcome::Err(_) => std::ptr::null_mut(),
            }
        }

        #[no_mangle]
        pub extern "C" fn outcome_drop(outcome: *mut Outcome) {
            let _ = unsafe {
                // Safety: expecting a valid pointer from input.
                Box::from_raw(outcome)
            };
        }

        #[no_mangle]
        pub extern "C" fn dump_get_len(dump: *const Vec<u8>) -> usize {
            let dump = unsafe {
                // Safety: expecting a valid pointer from input.
                &*dump
            };
            dump.len()
        }

        #[no_mangle]
        pub extern "C" fn dump_get_ptr(dump: *const Vec<u8>) -> *const u8 {
            let dump = unsafe {
                // Safety: expecting a valid pointer from input.
                &*dump
            };
            dump.as_ptr()
        }

        #[no_mangle]
        pub extern "C" fn dump_drop(dump: *mut Vec<u8>) {
            let _ = unsafe {
                // Safety: expecting a valid pointer from input.
                Box::from_raw(dump)
            };
        }

        #[no_mangle]
        pub extern "C" fn method_def_drop(method: *mut c_char) {
            let _ = unsafe {
                // Safety: expecting a valid pointer from input.
                CString::from_raw(method)
            };
        }

        #[no_mangle]
        pub extern "C" fn extension_init() -> *const c_char {
            fn extension_init() -> String {
                let manifest = $crate::serde_json::json!({
                    "outcome": {
                        "fn_get_err": "outcome_get_err",
                        "fn_get_ok": "outcome_get_ok",
                        "fn_drop": "outcome_drop"
                    },
                    "dumped": {
                        "fn_get_ptr": "dump_get_ptr",
                        "fn_get_len": "dump_get_len",
                        "fn_drop": "dump_drop"
                    },
                    "resources": {$(
                        stringify!($ty): {
                            "fn_from_bytes": stringify!($ty).to_string() + "_from_bytes",
                            "fn_dump": stringify!($ty).to_string() + "_dump",
                            "fn_size": stringify!($ty).to_string() + "_size",
                            "fn_get_method_def": stringify!($ty).to_string() + "_get_method",
                            "fn_drop_method_def": "method_def_drop",
                            "fn_drop": stringify!($ty).to_string() + "_drop"
                        },
                    )*}
                });

                manifest.to_string()
            }

            std::panic::catch_unwind(|| {
                // This leak will never be un-leaked.
                let boxed = CString::new(extension_init())
                    .expect("json output shouldn't contain nul characters")
                    .into_boxed_c_str();
                let c_str = Box::leak(boxed);
                c_str.as_ptr()
            })
            .unwrap_or_else(|_| {
                eprintln!("extension initialization panicked. See stderr");
                std::ptr::null()
            })
        }

        $(
            $crate::resource! { $ty }
        )*
    };
}

#[macro_export]
macro_rules! resource {
    ($ty:ty) => {
        $crate::paste! {

            #[allow(unused)]
            fn test_is_a_resource() where $ty: $crate::Resource  {}

            #[no_mangle]
            pub unsafe extern "C" fn [<$ty _size>](raw: *mut $ty) -> usize {
                std::panic::catch_unwind(|| (&*raw).size())
                    .unwrap_or_else(|_| {
                        eprintln!(
                            "calling `size` on resource {:?} panicked. Size will be set to zero. See stderr.",
                            stringify!($ty)
                        );
                        0
                    })
            }

            #[no_mangle]
            pub unsafe extern "C" fn [<$ty _dump>](raw: *mut $ty) -> *mut $crate::Outcome {
                std::panic::catch_unwind(|| {
                    let boxed = Box::new($crate::Outcome::from((&*raw).dump()));
                    Box::leak(boxed) as *mut _
                }).unwrap_or_else(|_| {
                    eprintln!(
                        "calling `dump` on resource {:?} panicked. Will return null. See stderr.",
                        stringify!($ty)
                    );
                    std::ptr::null_mut()
                })
            }

            #[no_mangle]
            pub unsafe extern "C" fn [<$ty _from_bytes>](
                bytes_ptr: *const u8,
                bytes_len: usize,
            ) -> *mut $crate::Outcome {
                std::panic::catch_unwind(|| {
                    let bytes = std::slice::from_raw_parts(bytes_ptr, bytes_len);
                    let boxed = Box::new($crate::Outcome::from($ty::from_bytes(bytes)));
                    Box::leak(boxed) as *mut _
                }).unwrap_or_else(|_| {
                    eprintln!(
                        "calling `dump` on resource {:?} panicked. Will return null. See stderr.",
                        stringify!($ty)
                    );
                    std::ptr::null_mut()
                })
            }

            #[no_mangle]
            pub unsafe extern "C" fn [<$ty _get_method>](
                raw: *mut $ty,
                name: *const c_char,
            ) -> *const c_char {
                std::panic::catch_unwind(|| {
                    let name = std::ffi::CStr::from_ptr(name);
                    let method = (&*raw).get_method(&name.to_string_lossy());

                    if let Some(method) = method {
                        CString::new(
                            $crate::serde_json::to_string_pretty(&method)
                                .expect("can always serialize method as json")
                        )
                        .expect("json representation does not contain nul chars")
                        .into_raw()
                    } else {
                        std::ptr::null()
                    }
                }).unwrap_or_else(|_| {
                    eprintln!(
                        "calling `get_method` on resource {:?} panicked. See stderr.",
                        stringify!($ty)
                    );
                    std::ptr::null()
                })
            }

            #[no_mangle]
            pub unsafe extern "C" fn [<$ty _drop>](raw: *mut $ty) {
                std::panic::catch_unwind(|| {
                    let _ = Box::from_raw(raw);
                }).unwrap_or_else(|_| {
                    eprintln!(
                        "calling `drop` on resource {:?} panicked. See stderr.",
                        stringify!($ty)
                    );
                })
            }
        }
    };
}

/// A safe convenience macro for method call. This macro does three things for you:
/// 1. Converts the raw pointer to a reference.
/// 2. Converts the pointers into slices correctly.
/// 3. Treats possible panics, converting them to errors. Panics are always unwanted, but
/// panicking through an FFI boundary is UB. Therefore, this treatment is always necessary.
///
/// # Usage
///
/// ```
/// impl MyResource {
///     fn something_safe(
///         &self,
///         input: Input,
///         output: OutputBuilder,
///     ) -> Result<(), String> {   // or anything else implementing `ToString`...
///         // ...
///         todo!()
///     }
///
///     method!(something_safe)  // can only call from inside an impl block!
///                                   // This is for type safety reasons
/// }
///
/// ```
#[macro_export]
macro_rules! method {
    ($safe_interface:ident) => {
        $crate::paste! {
            pub unsafe extern "C" fn [<raw_method__ $safe_interface>](
                resource_ptr: *const (),
                input_ptr: *const u8,
                input_slots: u64,
                output_ptr: *mut u8,
                output_slots: u64,
            ) -> *mut $crate::FnError {
                match std::panic::catch_unwind(|| {
                    unsafe {
                        // Safety: all this stuff came from jyafn code. The jyafn code should
                        // provide valid parameters. Plus, it's the responsibility of the
                        // implmementer guarantee that the types match.

                        let resource: &Self = &*(resource_ptr as *const _);

                        Self::$safe_interface(
                            resource,
                            $crate::Input::new(input_ptr, input_slots as usize),
                            $crate::OutputBuilder::new(output_ptr, output_slots as usize),
                        )
                    }
                }) {
                    Ok(Ok(())) => std::ptr::null_mut(),
                    Ok(Err(err)) => {
                        let boxed = Box::new(err.to_string().into());
                        Box::leak(boxed)
                    }
                    // DON'T forget the nul character when working with bytes directly!
                    Err(_) => {
                        let boxed = Box::new("method panicked. See stderr".to_string().into());
                        Box::leak(boxed)
                    }
                }
            }
        }
    };
}

/// A convenience macro to get references to methods created with [`make_method`].
#[macro_export]
macro_rules! get_method_ptr {
    ($safe_interface:ident) => {
        $crate::paste!(Self::[<raw_method__ $safe_interface>]) as usize
    }
}

/// This macro provides a standard implementation for the [`Resource::get_method`]
/// function from a list of methods.
///
/// # Usage
///
/// ```
/// impl Resource for MyResource {
///     // ...
///
///     declare_methods! {
///         foo_method(x: scalar, y: [datetime; self.size]) -> [datetime; self.size]
///     }
/// }
/// ```
#[macro_export]
macro_rules! declare_methods {
    ($($safe_interface:ident ($($key:tt : $ty:tt),*) -> $output:tt )*) => {
        fn get_method(&self, method: &str) -> Option<$crate::Method> {
            Some(match method {
                $(
                    stringify!($safe_interface) => $crate::Method {
                        fn_ptr: $crate::get_method_ptr!($safe_interface),
                        input_layout: $crate::r#struct!($($key : $ty),*),
                        output_layout: $crate::layout!($output),
                    },
                )*
                _ => return None,
            })
        }
    }
}
