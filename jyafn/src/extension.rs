//! An extension is a wrapper over a shared object comforming to a given interface.

use libloading::{Library, Symbol};
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoadOutcome {
    // Extension loading failed
    Failed {
        /// The reason why it failed.
        error: String,
    },
    Loaded(Box<ExtensionManifest>),
}

/// This is the data format, returned as a C-style string from the `extension_init`
/// initialization function. This describes which symbols to be used by each resource in
/// this extension.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Tells us what this extension is.
    pub metadata: ExtensionMetadata,
    /// Describes the symbols to be used when accessing outcomes of fallible operations.
    pub outcome: OutcomeManifest,
    /// Describes the symbols to be used when accessing buffers of binary memory.
    pub dumped: DumpedManifest,
    /// Describes the symbols to be used when accessing C-style strings generated by this
    /// library.
    pub string: StringManifest,
    /// Describes the symbols to be used when interfacing with each resource type provided
    /// by this extension.
    pub resources: HashMap<String, ResourceManifest>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    /// This is the name of the extension and must match the file name in the filesystem.
    pub name: String,
    /// This is the version of the extension and must be a valid semantic version and
    /// must match the file name in the filesystem.
    #[serde_as(as = "DisplayFromStr")]
    pub version: semver::Version,
}

/// Lists the names of the symbols needed to create the interface between an outcome and
/// jyafn. See [`OutcomeSymbols`] for detailed information on the contract for each
/// symbol.
#[derive(Debug, Serialize, Deserialize)]
pub struct OutcomeManifest {
    pub fn_get_err: String,
    pub fn_get_ok: String,
    pub fn_drop: String,
}

/// Lists the names of the symbols needed to create the interface between a dump of
/// binary data and jyafn. See [`DumpedSymbols`] for detailed information on the
/// contract for each symbol.
#[derive(Debug, Serialize, Deserialize)]
pub struct DumpedManifest {
    pub fn_get_ptr: String,
    pub fn_get_len: String,
    pub fn_drop: String,
}

/// Lists the name of the symbols needed to create the interface between C-style strings
/// from the extension and jyafn. See [`StringSymbols`] for detailed information on the
/// contract for each symbol.
#[derive(Debug, Serialize, Deserialize)]
pub struct StringManifest {
    pub fn_drop: String,
}

/// Lists the names of the symbols needed to create the interface between a resource and
/// jyafn. See [`ResourceSymbols`] for detailed information on the contract for each
/// symbol.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceManifest {
    pub fn_from_bytes: String,
    pub fn_dump: String,
    pub fn_size: String,
    pub fn_get_method_def: String,
    pub fn_drop: String,
}

/// A declaration of an external method. Gives a raw function pointer to be called in
/// jyafn code accompanied by the layout of the input and the output of the function.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalMethod {
    /// The function pointer to be called in jyafn code.
    pub fn_ptr: usize,
    /// The input layout of the given function.
    pub input_layout: Struct,
    /// Output layout of the given function.
    pub output_layout: Layout,
}

/// Checks for nul chars in the provided string and returns a nul-termindated slice.
fn str_to_symbol_name(s: &str) -> Result<Vec<u8>, Error> {
    Ok(CString::new(s)
        .map_err(|err| err.to_string())?
        .into_bytes_with_nul())
}

/// Gets a symbol from a library. This returns a copy of the symbol, to skip the lifetime
/// checking mechanism of `libloading`. We will manually guarantee that the library is
/// always present in memory (up to and including using `std::mem::forget` if necessary).
unsafe fn get_symbol<T: Copy>(library: &Library, name: &str) -> Result<T, Error> {
    Ok(*library.get::<T>(&str_to_symbol_name(name)?)?)
}

/// Lists the names of the symbols needed to create the interface between an outcome and
/// jyafn.
#[derive(Debug)]
pub struct OutcomeSymbols {
    /// Returns a C-style string if the given outcome is an error. Else, it should return
    /// null. This function will be called only once per outcome.
    pub fn_get_err: unsafe extern "C" fn(Outcome) -> *const c_char,
    /// Returns the successful result if the given outcome is success. The value in case
    /// of an error is undetermined. This function will be called at most one per outcome.
    pub fn_get_ok: unsafe extern "C" fn(Outcome) -> *mut (),
    /// Drops any memory associated with this outcome. This function will be called at
    /// most once per outcome and after it, no more calls involving the current outcome
    /// will ever be performed.
    pub fn_drop: unsafe extern "C" fn(Outcome),
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

/// Lists the names of the symbols needed to create the interface between a buffer of
/// binary data jyafn.
#[derive(Debug)]
pub struct DumpedSymbols {
    /// Gets the starting pointer of the binary representation.
    pub fn_get_len: unsafe extern "C" fn(Dumped) -> usize,
    /// Gets the length of the binary representation.
    pub fn_get_ptr: unsafe extern "C" fn(Dumped) -> *const u8,
    /// Drops any allocated memory created for this given dump. Will be called only once
    /// per dump.
    pub fn_drop: unsafe extern "C" fn(Dumped),
}

impl DumpedSymbols {
    /// Loads the outcome symbols from the supplied library, given a manifest.
    unsafe fn load(library: &Library, manifest: &DumpedManifest) -> Result<DumpedSymbols, Error> {
        /// For building structs that are symbol tables.
        macro_rules! symbol {
            ($($sym:ident),*) => { Self {$(
                $sym: get_symbol(library, &manifest.$sym).context(
                        concat!("getting symbol for ", stringify!($sym)
                    )
                )?,
            )*}}
        }

        Ok(symbol!(fn_get_len, fn_get_ptr, fn_drop))
    }
}

/// Lists the name of the symbols needed to create the interface between C-style strings
/// from the extension and jyafn.
#[derive(Debug, Clone)]
pub struct StringSymbols {
    /// Drops any allocated memory created for the C-style string pointed by the input.
    /// This will be called only on pointers generated from within the extension (returned
    /// strings) and will be called only once per pointer.
    pub fn_drop: unsafe extern "C" fn(*mut c_char),
}

impl StringSymbols {
    /// Loads the resource symbols from the supplied library, given a manifest.
    unsafe fn load(library: &Library, manifest: &StringManifest) -> Result<StringSymbols, Error> {
        /// For building structs that are symbol tables.
        macro_rules! symbol {
            ($($sym:ident),*) => { Self {$(
                $sym: get_symbol(library, &manifest.$sym).context(
                        concat!("getting symbol for ", stringify!($sym)
                    )
                )?,
            )*}}
        }

        Ok(symbol!(fn_drop))
    }
}

/// Lists the names of the symbols needed to create the interface between a resource and
/// jyafn.
#[derive(Debug, Clone)]
pub struct ResourceSymbols {
    /// Creates a new resource from the supplied binary data and length. This is the same
    /// data that is returned by the `fn_dump` function.
    pub fn_from_bytes: unsafe extern "C" fn(*const u8, usize) -> Outcome,
    /// Creates a dump, which points to the binary representation of the supplied resource.
    pub fn_dump: unsafe extern "C" fn(RawResource) -> Outcome,
    /// Gets the amount of heap memory (ie RAM) allocated by this resource.
    pub fn_size: unsafe extern "C" fn(RawResource) -> usize,
    /// Given the `name` of a method and its `config` (i.e., aditional parameters) as
    /// C-style strings, returns the JSON representation of an [`ExternalMethod`] as a
    /// C-style string.
    pub fn_get_method_def: unsafe extern "C" fn(RawResource, *const c_char) -> *mut c_char,
    /// Drops any allocation memory created for this resource. This will be called only
    /// once per resource and, after this call, no more calls are expected on the given
    /// resource.
    pub fn_drop: unsafe extern "C" fn(RawResource),
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
            fn_size,
            fn_get_method_def,
            fn_drop
        ))
    }
}

type LoadedExtensionVersions = HashMap<semver::Version, Arc<Extension>>;

lazy_static::lazy_static! {
    static ref EXTENSIONS: RwLock<HashMap<String, LoadedExtensionVersions>> =
        RwLock::default();
}

/// An extension is a wrapper over a shared object comforming to a given interface. This
/// can be used to create extra "resources" that can be accessed from jyafn. It's useful
/// when interacting with systems that would otherwise be very difficult to interact
/// with in jyafn, but for which (normally) a C wrapper (or something of that sort) is
/// readlily available.
#[derive(Debug)]
pub struct Extension {
    /// The shared object handle.
    _library: Library,
    /// The metedata of this extension.
    metadata: ExtensionMetadata,
    /// Describes the symbols to be used when accessing outcomes of fallible operations.
    outcome: OutcomeSymbols,
    /// Describes the symbols to be used when accessing buffers of binary memory.
    dumped: DumpedSymbols,
    /// Describes the symbols to be used when accessing C-style strings generated by this
    /// library.
    pub string: StringSymbols,
    /// Describes the symbols to be used when interfacing with each resource type provided
    /// by this extension.
    resources: HashMap<String, ResourceSymbols>,
}

impl Extension {
    /// Loads an extension, given a path. This path is OS-specific and will be resolved
    /// by the OS acording to its own quirky rules.
    pub(crate) fn load(path: &Path) -> Result<Extension, Error> {
        unsafe {
            // Safety: we can only pray nobody loads anything funny here. However, it's
            // not my responsibilty what kind of crap you install in your computer.
            let library = Library::new(path)?;
            let extension_init: Symbol<ExtensionInit> = library.get(EXTENSION_INIT_SYMBOL)?;
            let outcome = extension_init();
            if outcome.is_null() {
                return Err(format!("library {path:?} failed to load").into());
            }
            let parsed: LoadOutcome = serde_json::from_slice(CStr::from_ptr(outcome).to_bytes())
                .map_err(|err| err.to_string())?;

            let manifest = match parsed {
                LoadOutcome::Loaded(manifest) => manifest,
                LoadOutcome::Failed { error } => return Err(error.into()),
            };

            let string = StringSymbols::load(&library, &manifest.string)
                .with_context(|| format!("loading `string` symbols from {path:?}"))?;
            let fn_drop = string.fn_drop;
            scopeguard::defer! {
                (fn_drop)(outcome as *mut i8);
            }

            let outcome = OutcomeSymbols::load(&library, &manifest.outcome)
                .with_context(|| format!("loading `outcome` symbols from {path:?}"))?;

            let dumped = DumpedSymbols::load(&library, &manifest.dumped)
                .with_context(|| format!("loading `dumped` symbols from {path:?}"))?;

            let resources = manifest
                .resources
                .iter()
                .map(|(name, resource)| {
                    Ok((
                        name.clone(),
                        ResourceSymbols::load(&library, resource)
                            .with_context(|| format!("loading resource {name:?} from {path:?}"))?,
                    ))
                })
                .collect::<Result<_, Error>>()?;

            Ok(Extension {
                _library: library,
                metadata: manifest.metadata,
                outcome,
                dumped,
                string,
                resources,
            })
        }
    }

    /// Gets a raw `Outcome` pointer and makes it into a result.
    pub(crate) unsafe fn outcome_to_result(&self, outcome: Outcome) -> Result<*mut (), Error> {
        unsafe {
            // Safety: supposing that the extension is correctly implmented and observing
            // the contract.
            let maybe_err = (self.outcome.fn_get_err)(outcome);
            let result = if !maybe_err.is_null() {
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

    pub(crate) unsafe fn dumped_to_vec(&self, dumped: Dumped) -> Result<Vec<u8>, Error> {
        unsafe {
            // Safety: supposing that the extension is correctly implmented and observing
            // the contract.
            scopeguard::defer! {
                (self.dumped.fn_drop)(dumped)
            }

            let dump_ptr = (self.dumped.fn_get_ptr)(dumped);
            if dump_ptr.is_null() {
                return Err("dump location was null".to_string().into());
            }
            let dump_len = (self.dumped.fn_get_len)(dumped);

            Ok(std::slice::from_raw_parts(dump_ptr, dump_len).to_vec())
        }
    }

    pub(crate) fn get_resource(&self, name: &str) -> Option<ResourceSymbols> {
        self.resources.get(name).cloned()
    }

    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn version(&self) -> &semver::Version {
        &self.metadata.version
    }

    pub fn resources(&self) -> impl Iterator<Item = &str> {
        self.resources.keys().map(|key| key.as_str())
    }
}

#[cfg(target_os = "linux")]
const SO_EXTENSION: &str = "so";
#[cfg(target_os = "macos")]
const SO_EXTENSION: &str = "dylib";
#[cfg(target_os = "windows")]
const SO_EXTENSION: &str = "dll";

/// Tests whether an extension name is valid or not. Currently allowed are lowercase
/// ascii, digit ascii or underline, with the first letter being a lowercase.
fn test_valid_name(name: &str) -> Result<(), Error> {
    let is_valid = name.starts_with(|ch: char| ch.is_ascii_lowercase())
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');

    if !is_valid {
        return Err(format!("extension name {name:?} is invalid").into());
    }

    Ok(())
}

/// Resolves the nice little name of the library into an ugly path that dlopen can
/// understand.
fn resolve_name(
    name: &str,
    version_req: &semver::VersionReq,
) -> Result<(semver::Version, PathBuf), Error> {
    let full_path = std::env::var("JYAFN_PATH").unwrap_or_else(|_| {
        home::home_dir()
            .map(|home| home.join(".jyafn/extensions").to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let mut tried = vec![];
    for alternative in full_path.split(',') {
        let alternative = alternative.trim();
        let mut candidates = vec![];
        let glob = format!("{alternative}/{name}-*.{SO_EXTENSION}");

        for path in glob::glob(&glob).map_err(|err| err.to_string())? {
            let path = path.map_err(glob::GlobError::into_error)?;
            if path.extension() != Some(SO_EXTENSION.as_ref()) {
                // not a shared object
                continue;
            }
            let Some(filename_os) = path.file_stem() else {
                // no file stem
                continue;
            };
            let filename = filename_os.to_string_lossy();
            let Some(version) = filename.split('-').last() else {
                // no version part
                tried.push(format!("{path:?}"));
                continue;
            };
            let Ok(semver) = version.parse::<semver::Version>() else {
                // not a valid semver
                tried.push(format!("{path:?}"));
                continue;
            };

            if version_req.matches(&semver) {
                candidates.push((semver, path));
            } else {
                // version doesn't match requirements
                tried.push(format!("{path:?}"));
            }
        }

        if let Some(best_candidate) = candidates
            .into_iter()
            .max_by_key(|(semver, _)| semver.clone())
        {
            return Ok(best_candidate);
        }
    }

    Err(format!(
        "failed to resolve extension {name:?} (tried {})",
        tried.join(", ")
    )
    .into())
}

/// Loads an extension, if it was not loaded before.
pub fn try_get(name: &str, version_req: &semver::VersionReq) -> Result<Arc<Extension>, Error> {
    test_valid_name(name)?;
    let (version, path) = resolve_name(name, version_req)?;

    let mut lock = EXTENSIONS.write().expect("poisoned");
    let loaded_extensions = lock.entry(name.to_owned()).or_default();
    if let Some(extension) = loaded_extensions.get(&version) {
        return Ok(extension.clone());
    }

    let extension =
        Arc::new(Extension::load(&path).with_context(|| format!("loading extension {name:?}"))?);

    // Check if what you got is what was actually advertised:
    if extension.metadata.name != name {
        return Err(format!(
            "file {path:?} should provide {name:?} but provides {:?}",
            extension.metadata.name
        )
        .into());
    }
    if extension.metadata.version != version {
        return Err(format!(
            "file {path:?} should provide version {version} but provides {}",
            extension.metadata.version
        )
        .into());
    }

    loaded_extensions.insert(version, extension.clone());

    Ok(extension)
}

/// Gets an extension by its name, panicking if it was not loaded.
pub fn get(name: &str, version_req: &semver::VersionReq) -> Arc<Extension> {
    try_get(name, version_req).expect("extension not loaded")
}

/// Lists the names and versions of all currently loaded extensions.
pub fn list() -> HashMap<String, Vec<semver::Version>> {
    EXTENSIONS
        .read()
        .expect("poisoned")
        .iter()
        .map(|(name, versions)| (name.clone(), versions.keys().cloned().collect::<Vec<_>>()))
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_load_extension() {
        get("dummy", &"*".parse().unwrap());
    }
}
