//! An extension is a wrapper over a shared object comforming to a given interface.

use lazy_static::lazy_static;
use libloading::{Library, Symbol};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::sync::{Arc, RwLock};

use crate::layout::{Layout, Struct};
use crate::{Context, Error};

/// The initialization function for the extension. This function should return a JSON
/// enconded [`ExtensionManifest`] or a null pointer in case of an error.
pub type ExtensionInit = unsafe extern "C" fn() -> *const c_char;

/// The name of the initalization function for the extension.
pub const EXTENSION_INIT_SYMBOL: &[u8] = b"extension_init\0";

/// A result of a falible operation made by the extension. This is just a `void*`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome(pub(crate) *mut ());

/// The representation of a resource. This is just a `void*`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawResource(pub(crate) *mut ());

/// The representation of a resource dump. This is just a `void*`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dumped(pub(crate) *mut ());

/// This is the data format, returned as a C-style string from the `extension_init`
/// initialization function. This describes which symbols to be used by each resource in
/// this extension.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Describes the symbols to be used when accessing outcomes of fallible operations.
    outcome: OutcomeManifest,
    /// Describes the symbols to be used when interfacing with each resource type provided
    /// by this extension.
    resources: HashMap<String, ResourceManifest>,
}

/// Lists the names of the symbols needed to create the interface between an outcome and
/// jyafn. See [`OutcomeSymbols`] for detailed information on the contract for each
/// symbol.
#[derive(Debug, Serialize, Deserialize)]
pub struct OutcomeManifest {
    fn_get_err: String,
    fn_get_ok: String,
    fn_drop: String,
}

/// Lists the names of the symbols needed to create the interface between a resource and
/// jyafn. See [`ResourceSymbols`] for detailed information on the contract for each
/// symbol.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceManifest {
    fn_from_bytes: String,
    fn_dump: String,
    fn_dump_ptr: String,
    fn_dump_len: String,
    fn_drop_dump: String,
    fn_size: String,
    fn_get_method_def: String,
    fn_drop_method_def: String,
    fn_drop: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalMethod {
    pub(crate) fn_ptr: usize,
    pub(crate) input_layout: Struct,
    pub(crate) output_layout: Layout,
}

/// Checks for nul chars in the provided string and returns a nul-termindated slice.
fn str_to_symbol_name(s: &str) -> Result<&[u8], Error> {
    Ok(CStr::from_bytes_until_nul(s.as_bytes())
        .map_err(|err| err.to_string())?
        .to_bytes_with_nul())
}

/// Gets a symbol from a library. This returns a copy of the symbol, to skip the lifetime
/// checking mechanism of `libloading`. We will manually guarantee that the library is
/// always present in memory (up to and including using `std::mem::forget` if necessary).
unsafe fn get_symbol<T: Copy>(library: &Library, name: &str) -> Result<T, Error> {
    Ok(*library.get::<T>(str_to_symbol_name(name)?)?)
}

/// Lists the names of the symbols needed to create the interface between an outcome and
/// jyafn.
#[derive(Debug)]
pub struct OutcomeSymbols {
    /// Returns a C-style string if the given outcome is an error. Else, it should return
    /// null. This function will be called only once per outcome.
    fn_get_err: unsafe extern "C" fn(Outcome) -> *const c_char,
    /// Returns the successful result if the given outcome is success. The value in case
    /// of an error is undetermined. This function will be called at most one per outcome.
    fn_get_ok: unsafe extern "C" fn(Outcome) -> *mut (),
    /// Drops any memory associated with this outcome. This function will be called at
    /// most once per outcome and after it, no more calls involving the current outcome
    /// will ever be performed.
    fn_drop: unsafe extern "C" fn(Outcome),
}

impl OutcomeSymbols {
    /// Loads the outcome symbols from the supplied library, given a manifest.
    unsafe fn load(library: &Library, manifest: &OutcomeManifest) -> Result<OutcomeSymbols, Error> {
        /// For building structs that are symbol tables.
        macro_rules! symbol {
            ($($sym:ident),*) => { Self {$(
                $sym: get_symbol(library, &manifest.$sym).context(
                        concat!("getting symbol for ", stringify!($sym)
                    )
                )?,
            )*}}
        }

        Ok(symbol!(fn_get_err, fn_get_ok, fn_drop))
    }
}

/// Lists the names of the symbols needed to create the interface between a resource and
/// jyafn.
#[derive(Debug, Clone)]
pub(crate) struct ResourceSymbols {
    /// Creates a new resource from the supplied binary data and length. This is the same
    /// data that is returned by the `fn_dump` function.
    pub(crate) fn_from_bytes: unsafe extern "C" fn(*const u8, usize) -> Outcome,
    /// Creates a dump, which points to the binary representation of the supplied resource.
    pub(crate) fn_dump: unsafe extern "C" fn(RawResource) -> Dumped,
    /// Gets the starting pointer of the binary representation.
    pub(crate) fn_dump_len: unsafe extern "C" fn(Dumped) -> usize,
    /// Gets the length of the binary representation.
    pub(crate) fn_dump_ptr: unsafe extern "C" fn(Dumped) -> *const u8,
    /// Drops any allocated memory created for this given dump. Will be called only once
    /// per dump.
    pub(crate) fn_drop_dump: unsafe extern "C" fn(Dumped),
    /// Gets the amount of heap memory (ie RAM) allocated by this resource.
    pub(crate) fn_size: unsafe extern "C" fn(RawResource) -> usize,
    /// Given the `name` of a method and its `config` (i.e., aditional parameters) as
    /// C-style strings, returns the JSON representation of an [`ExternalMethod`] as a
    /// C-style string.
    pub(crate) fn_get_method_def: unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char,
    /// Drops any allocated memory created for this given method definition. Will be
    /// called only once per method definiton created by `fn_get_method_def`.
    pub(crate) fn_drop_method_def: unsafe extern "C" fn(*mut c_char),
    /// Drops any allocation memory created for this resource. This will be called only
    /// once per resource and, after this call, no more calls are expected on the given
    /// resource.
    pub(crate) fn_drop: unsafe extern "C" fn(RawResource),
}

impl ResourceSymbols {
    /// Loads the resource symbols from the supplied library, given a manifest.
    unsafe fn load(
        library: &Library,
        manifest: &ResourceManifest,
    ) -> Result<ResourceSymbols, Error> {
        /// For building structs that are symbol tables.
        macro_rules! symbol {
            ($($sym:ident),*) => { Self {$(
                $sym: get_symbol(library, &manifest.$sym).context(
                        concat!("getting symbol for ", stringify!($sym)
                    )
                )?,
            )*}}
        }

        Ok(symbol!(
            fn_from_bytes,
            fn_dump,
            fn_dump_ptr,
            fn_dump_len,
            fn_drop_dump,
            fn_size,
            fn_get_method_def,
            fn_drop_method_def,
            fn_drop
        ))
    }
}

lazy_static! {
    static ref EXTENSIONS: RwLock<HashMap<String, Arc<Extension>>> = RwLock::default();
}

/// An extension is a wrapper over a shared object comforming to a given interface. This
/// can be used to create extra "resources" that can be accessed from jyafn. It's useful
/// when interacting with systems that would otherwise be very difficult to interact
/// with in jyafn, but for which (normally) a C wrapper (or something of that sort) is
/// readlily available.
pub struct Extension {
    /// The shared object handle.
    _library: Library,
    /// Describes the symbols to be used when accessing outcomes of fallible operations.
    outcome: OutcomeSymbols,
    /// Describes the symbols to be used when interfacing with each resource type provided
    /// by this extension.
    resources: HashMap<String, ResourceSymbols>,
}

impl Extension {
    /// Loads an extension, given a path. This path is OS-specific and will be resolved
    /// by the OS acording to its own quirky rules.
    pub(crate) fn load(path: &str) -> Result<Extension, Error> {
        unsafe {
            // Safety: we can only pray nobody loads anything funny here. However, it's
            // not my responsibilty what kind of crap you install in your computer.
            let library = Library::new(path)?;
            let extension_init: Symbol<ExtensionInit> = library.get(EXTENSION_INIT_SYMBOL)?;
            let outcome = extension_init();
            if outcome == std::ptr::null_mut() {
                return Err(format!("library at {path} failed to load").into());
            }
            let manifest: ExtensionManifest =
                serde_json::from_slice(CStr::from_ptr(outcome).to_bytes())
                    .map_err(|err| err.to_string())?;

            let outcome = OutcomeSymbols::load(&library, &manifest.outcome)
                .with_context(|| format!("loading outcome symbols from {path}"))?;

            let resources = manifest
                .resources
                .iter()
                .map(|(name, resource)| {
                    Ok((
                        name.clone(),
                        ResourceSymbols::load(&library, resource)
                            .with_context(|| format!("loading resource {name} from {path}"))?,
                    ))
                })
                .collect::<Result<_, Error>>()?;

            Ok(Extension {
                _library: library,
                outcome,
                resources,
            })
        }
    }

    /// Gets a raw `Outcome` pointer and makes it into a result. This method is considered safe because
    pub(crate) unsafe fn outcome_to_result(&self, outcome: Outcome) -> Result<*mut (), Error> {
        unsafe {
            // Safety: supposing that the extension is correctly implmented and observing
            // the contract.
            let maybe_err = (self.outcome.fn_get_err)(outcome);
            let result = if maybe_err != std::ptr::null() {
                Err(CStr::from_ptr(maybe_err)
                    .to_string_lossy()
                    .to_string()
                    .into())
            } else {
                Ok((self.outcome.fn_get_ok)(outcome))
            };

            scopeguard::defer! {
                (self.outcome.fn_drop)(outcome);
            }

            result
        }
    }

    pub(crate) fn get_resource(&self, name: &str) -> Option<ResourceSymbols> {
        self.resources.get(name).cloned()
    }
}

/// Resolves the nice little name of the library into an ugly path that dlopen can
/// understand.
fn resolve_name(name: &str) -> Result<String, Error> {
    // TODO: something fancier than this...
    Ok(name.to_owned())
}

/// Loads an extension, if it was not loaded before.
pub fn load(name: &str) -> Result<(), Error> {
    let mut lock = EXTENSIONS.write().expect("poisoned");

    if lock.contains_key(name) {
        // already loaded
        return Ok(());
    }

    let path = resolve_name(name)?;
    let extension = Extension::load(&path).with_context(|| format!("loading extension {name}"))?;

    lock.insert(name.to_owned(), Arc::new(extension));

    Ok(())
}

/// Gets an extension by its name, returning `None` if it was not loaded.
pub fn get_opt(name: &str) -> Option<Arc<Extension>> {
    EXTENSIONS.read().expect("poisoned").get(name).cloned()
}

/// Gets an extension by its name, panicking if it was not loaded.
pub fn get(name: &str) -> Arc<Extension> {
    get_opt(name).expect("extension not loaded")
}
