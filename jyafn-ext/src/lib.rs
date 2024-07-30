//! This crate is intended to help extension authors. It exposes a minimal version of
//! `jyafn` and many convenience macros to generate all the boilerplate involved.

mod io;
mod layout;
mod outcome;
mod resource;

/// Reexporting from the `paste` crate. This is neeede because we have to programatically
/// generate new identifiers to be used as symbols in the final shared object.
pub use paste::paste;
/// We need JSON support to zip JSON values around the FFI boundary.
pub use serde_json;

pub use io::{Input, OutputBuilder};
pub use layout::{Layout, Struct, ISOFORMAT};
pub use outcome::Outcome;
pub use resource::{Method, Resource};

/// Generates the boilerplate code for a `jyafn` extension.
///
/// # Usage
///
/// This macro accepts a list of comman-separated types, each of which has to implement
/// the [`Resource`] trait, like so
/// ```
/// extension! {
///     Foo, Bar, Baz
/// }
/// ```
/// Optionally, you may define an init function, which takes no arguments and returns
/// `Result<(), String>`, like so
/// ```
/// extension! {
///     init = my_init;
///     Foo, Bar, Baz
/// }
///
/// fn my_init() -> Result<(), String> { /* ... */}
/// ```
#[macro_export]
macro_rules! extension {
    ($($ty:ty),*) => {
        fn noop() -> Result<(), String> { Ok (()) }

        $crate::extension! {
            init = noop;
            $($ty),*
        }
    };
    (init = $init_fn:ident; $($ty:ty),*) => {
        use std::ffi::{c_char, CString};
        use $crate::Outcome;

        /// Creates a C-style string out of a `String` in a way that doesn't produce errors. This
        /// function substitutes nul characters by the ` ` (space) character. This avoids an
        /// allocation.
        ///
        /// This method **leaks** the string. So, don't forget to guarantee that somene somewhere
        /// is freeing it.
        ///
        /// # Note
        ///
        /// Yes, I know! It's a pretty lousy implementation that is even... O(n^2) (!!). You can
        /// do better than I in 10mins.
        pub(crate) fn make_safe_c_str(s: String) -> CString {
            let mut v = s.into_bytes();
            loop {
                match std::ffi::CString::new(v) {
                    Ok(c_str) => return c_str,
                    Err(err) => {
                        let nul_position = err.nul_position();
                        v = err.into_vec();
                        v[nul_position] = b' ';
                    }
                }
            }
        }


        /// # Safety
        ///
        /// Expecting a valid pointer from input.
        #[no_mangle]
        pub unsafe extern "C" fn outcome_get_err(outcome: *mut Outcome) -> *const c_char {
            let outcome = &*outcome;

            match outcome {
                Outcome::Ok(_) => std::ptr::null(),
                Outcome::Err(err) => err.as_ptr(),
            }
        }

        /// # Safety
        ///
        /// Expecting a valid pointer from input.
        #[no_mangle]
        pub unsafe extern "C" fn outcome_get_ok(outcome: *mut Outcome) -> *mut () {
            let outcome = &*outcome;

            match outcome {
                Outcome::Ok(ptr) => *ptr,
                Outcome::Err(_) => std::ptr::null_mut(),
            }
        }

        /// # Safety
        ///
        /// Expecting a valid pointer from input.
        #[no_mangle]
        pub unsafe extern "C" fn outcome_drop(outcome: *mut Outcome) {
            let _ = Box::from_raw(outcome);
        }

        /// # Safety
        ///
        /// Expecting a valid pointer from input.
        #[no_mangle]
        pub unsafe extern "C" fn dump_get_len(dump: *const Vec<u8>) -> usize {
            let dump = &*dump;
            dump.len()
        }

        /// # Safety
        ///
        /// Expecting a valid pointer from input.
        #[no_mangle]
        pub unsafe extern "C" fn dump_get_ptr(dump: *const Vec<u8>) -> *const u8 {
            let dump = &*dump;
            dump.as_ptr()
        }

        /// # Safety
        ///
        /// Expecting a valid pointer from input.
        #[no_mangle]
        pub unsafe extern "C" fn dump_drop(dump: *mut Vec<u8>) {
            let _ = Box::from_raw(dump);
        }

        #[no_mangle]
        pub unsafe extern "C" fn string_drop(method: *mut c_char) {
            let _ = CString::from_raw(method);
        }

        #[no_mangle]
        pub extern "C" fn extension_init() -> *const c_char {
            fn safe_extension_init() -> Result<$crate::serde_json::Value, String> {
                $init_fn()?;

                let manifest = $crate::serde_json::json!({
                    "metadata": {
                        "name": env!("CARGO_PKG_NAME"),
                        "version": env!("CARGO_PKG_VERSION"),
                        "about": env!("CARGO_PKG_DESCRIPTION"),
                        "authors": env!("CARGO_PKG_AUTHORS"),
                        "license": env!("CARGO_PKG_LICENSE"),
                    },
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
                    "string": {
                        "fn_drop": "string_drop"
                    },
                    "resources": {$(
                        stringify!($ty): {
                            "fn_from_bytes": stringify!($ty).to_string() + "_from_bytes",
                            "fn_dump": stringify!($ty).to_string() + "_dump",
                            "fn_size": stringify!($ty).to_string() + "_size",
                            "fn_get_method_def": stringify!($ty).to_string() + "_get_method",
                            "fn_drop": stringify!($ty).to_string() + "_drop"
                        },
                    )*}
                });

                Ok(manifest)
            }

            let outcome = std::panic::catch_unwind(|| {
                match safe_extension_init() {
                    Ok(manifest) => manifest,
                    Err(err) => {
                        $crate::serde_json::json!({"error": err})
                    }
                }
            }).unwrap_or_else(|_| {
                $crate::serde_json::json!({
                    "error": "extension initialization panicked. See stderr"
                })
            });

            match CString::new(outcome.to_string()) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null(),
            }
        }

        $(
            $crate::resource! { $ty }
        )*
    };
}

/// Declares a single resource for this extension, given a type. This writes all the
/// boilerplate code thar corresponds to the extension side of the API.
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
///    panicking through an FFI boundary is UB. Therefore, this treatment is always necessary.
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
///                              // This is for type safety reasons
/// }
///
/// ```
#[macro_export]
macro_rules! method {
    ($safe_interface:ident) => {
        $crate::paste! {
            #[allow(non_snake_case)]
            pub unsafe extern "C" fn [<raw_method__ $safe_interface>](
                resource_ptr: *const (),
                input_ptr: *const u8,
                input_slots: u64,
                output_ptr: *mut u8,
                output_slots: u64,
            ) -> *mut u8 {
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
                        make_safe_c_str(err).into_raw() as *mut u8
                    }
                    // DON'T forget the nul character when working with bytes directly!
                    Err(_) => {
                        make_safe_c_str(format!(
                            "method {:?} panicked. See stderr",
                            stringify!($safe_interface),
                        )).into_raw() as *mut u8
                    }
                }
            }
        }
    };
}

/// A convenience macro to get references to methods created with [`method`].
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
///     fn get_method(&self, method: &str) -> Option<Method> {
///         declare_methods! {
///             // This the the variable containing the method name.
///             match method:
///                 // Use the layout notation to declare the method (an yes, you can use
///                 // `self` anywhere in the declaration)
///                 foo_method(x: scalar, y: [datetime; self.size]) -> [datetime; self.size];
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! declare_methods {
    ($( $safe_interface:ident ($($key:tt : $ty:tt),*) -> $output:tt; )*) => {
        $crate::declare_methods! {
            match method:  $( $safe_interface ($($key : $ty),*) -> $output; )*
        }
    };
    ( match $method:ident : $( $safe_interface:ident ($($key:tt : $ty:tt),*) -> $output:tt; )*) => {
        Some(match $method {
            $(
                stringify!($safe_interface) => $crate::Method {
                    fn_ptr: $crate::get_method_ptr!($safe_interface),
                    input_layout: $crate::r#struct!($($key : $ty),*),
                    output_layout: $crate::layout!($output),
                },
            )*
            _ => return None,
        })
    };
}
