//! A `Resource` is an amount of data associated with "methods", much like an object in
//! OO languages, but simpler.

pub mod dummy;
pub mod external;

use byte_slice_cast::*;
use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::io::Read;
use std::mem::MaybeUninit;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::pin::Pin;
use std::sync::Arc;
use zip::read::ZipFile;

use crate::layout::{Layout, Struct};
use crate::Error;

/// The signature of the function that will be invoked from inside the function code.
pub type RawResourceMethod =
    unsafe extern "C" fn(*const (), *const u8, u64, *mut u8, u64) -> *mut u8;

/// A method from a resource.
#[derive(Debug)]
pub struct ResourceMethod {
    /// The bare function that will be invoked from inside the function code.
    pub(crate) fn_ptr: RawResourceMethod,
    /// The input layout for the method.
    pub(crate) input_layout: Struct,
    /// The output layout for the method.
    pub(crate) output_layout: Layout,
}

/// A `ResourceType` creates resources of a given type. Think of this as the "class
/// object" of resources.
#[typetag::serde(tag = "type")]
pub trait ResourceType: std::fmt::Debug + Send + Sync + UnwindSafe + RefUnwindSafe {
    /// Creates a resource out of binary data.
    #[allow(clippy::wrong_self_convention)]
    fn from_bytes(&self, bytes: &[u8]) -> Result<Pin<Box<dyn Resource>>, Error>;

    /// Reads a resource from a zip file entry.
    ///
    /// Override this method if you know a more efficient of loading the resource other
    /// than reading the file to a buffer and then parsing the resulting buffer.
    fn read(&self, mut f: ZipFile<'_>) -> Result<Pin<Box<dyn Resource>>, Error> {
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        self.from_bytes(&buffer)
    }
}

/// A `Resource` is an amount of data associated with "methods", much like an object in
/// OO languages, but simpler. Specifically, resources shoud _not_ (ever!) support
/// mutation. Resources are immutable pices of data.
pub trait Resource: 'static + std::fmt::Debug + Send + Sync + UnwindSafe + RefUnwindSafe {
    /// Returns the type of this resource. This has to be the same value that, if applied
    /// to the output of `Resource:dump`, will again yield this exact resource.
    fn r#type(&self) -> Arc<dyn ResourceType>;
    /// Dumps this resource as binary data.
    fn dump(&self) -> Result<Vec<u8>, Error>;
    /// The ammount of heap used by this storage.
    fn size(&self) -> usize;
    /// Gets information on a method name for this resource, if it exists.
    fn get_method(&self, method: &str) -> Option<ResourceMethod>;

    /// The raw pointer to be used in jyafn code. Just override this method if you know
    /// _very well_ what you are doing.
    fn get_raw_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// A holder of a resource.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceContainer {
    /// The type of the contained resource.
    resource_type: Arc<dyn ResourceType>,
    /// The contained resource.
    ///
    /// We need this field because we _hardcode_ this pointer in the function code. If
    /// this moves anywhere, we get the pleasure of accessing bad memory and The Most
    /// Horrible Things™ ensue.
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[serde(default)]
    resource: Option<Pin<Box<dyn Resource>>>,
}

impl GetSize for ResourceContainer {
    fn get_heap_size(&self) -> usize {
        if let Some(resource) = &self.resource {
            resource.size()
        } else {
            0
        }
    }
}

impl ResourceContainer {
    /// Creates a new initialized container for the given resource.
    pub fn new<R: Resource>(resource: R) -> ResourceContainer {
        ResourceContainer {
            resource_type: resource.r#type(),
            resource: Some(Box::pin(resource)),
        }
    }

    /// Creates a new initialized container for the given boxed resource.
    pub fn new_boxed(resource: Pin<Box<dyn Resource>>) -> ResourceContainer {
        ResourceContainer {
            resource_type: resource.r#type(),
            resource: Some(resource),
        }
    }

    /// Reads the resource from a zip file entry.
    pub(crate) fn read(&self, f: ZipFile<'_>) -> Result<Self, Error> {
        let resource = self.resource_type.read(f)?;
        Ok(ResourceContainer {
            resource_type: self.resource_type.clone(),
            resource: Some(resource),
        })
    }

    /// Dumps this resource as binary information.
    pub(crate) fn dump(&self) -> Result<Vec<u8>, Error> {
        self.resource
            .as_ref()
            .expect("resource not initialized")
            .dump()
    }

    /// Checks whether this container was already initialized with a resource.
    pub fn is_initialized(&self) -> bool {
        self.resource.is_some()
    }

    pub fn get_raw_ptr(&self) -> *const () {
        self.resource
            .as_ref()
            .expect("resource not initialized")
            .get_raw_ptr()
    }

    /// Gets the underlying resource as a dynamic pointer. This function panics if the
    /// resource is not initialized.
    pub fn resource(&self) -> Pin<&dyn Resource> {
        self.resource
            .as_ref()
            .expect("resource not initialized")
            .as_ref()
    }

    /// Gets a information on a method for the containted resource, if it exists.
    pub fn get_method(&self, method: &str) -> Option<ResourceMethod> {
        self.resource
            .as_ref()
            .expect("resource not initialized")
            .get_method(method)
    }
}

/// A convenience wrapper over the input data pointer, given the information on its size.
#[repr(transparent)]
pub struct Input<'a>(&'a [u64]);

impl<'a> Input<'a> {
    /// Creates a new input.
    ///
    /// # Safety
    ///
    /// Make sure that `input` points to a slice with _memory size_ of `8 * n_slots` at
    /// least. Failing to do so, reads from bad memory may occur.
    pub unsafe fn new(input: *const u8, n_slots: usize) -> Self {
        Self(std::slice::from_raw_parts(input as *const u64, n_slots))
    }

    pub fn get_f64(&self, idx: usize) -> f64 {
        f64::from_ne_bytes(self.0[idx].to_ne_bytes())
    }

    pub fn get_u64(&self, idx: usize) -> u64 {
        self.0[idx]
    }

    pub fn get_bool(&self, idx: usize) -> bool {
        self.0[idx] == 1
    }

    pub fn as_f64_slice(&self) -> &[f64] {
        self.0
            .as_byte_slice()
            .as_slice_of()
            .expect("f64 and u64 have the same size")
    }

    pub fn as_u64_slice(&self) -> &[u64] {
        self.0
    }
}

/// A convenience wrapper over the output data pointer, given the information on its size.
pub struct OutputBuilder<'a> {
    position: usize,
    slice: &'a mut [MaybeUninit<u64>],
}

impl<'a> Drop for OutputBuilder<'a> {
    fn drop(&mut self) {
        // This prevents any uninitialized memory from ever being read.
        while self.position < self.slice.len() {
            self.push_u64(0)
        }
    }
}

impl<'a> OutputBuilder<'a> {
    /// Creates a new input.
    ///
    /// # Safety
    ///
    /// Make sure that `output` points to a slice with _memory size_ of `8 * n_slots` at
    /// least. Failing to do so, writes to bad memory may occur.
    pub unsafe fn new(output: *mut u8, n_slots: usize) -> Self {
        Self {
            position: 0,
            slice: std::slice::from_raw_parts_mut(output as *mut MaybeUninit<u64>, n_slots),
        }
    }

    pub fn push_f64(&mut self, val: f64) {
        self.slice[self.position].write(u64::from_ne_bytes(val.to_ne_bytes()));
        self.position += 1;
    }

    pub fn push_u64(&mut self, val: u64) {
        self.slice[self.position].write(val);
        self.position += 1;
    }

    pub fn push_bool(&mut self, val: bool) {
        self.slice[self.position].write(val as u64);
        self.position += 1;
    }

    pub fn copy_from_f64(&mut self, src: &[f64]) {
        for &val in src {
            self.push_f64(val);
        }
    }

    pub fn copy_from_u64(&mut self, src: &[u64]) {
        for &val in src {
            self.push_u64(val);
        }
    }

    pub fn copy_from_bool(&mut self, src: &[bool]) {
        for &val in src {
            self.push_bool(val);
        }
    }
}

/// A safe convenience macro for method call. This macro does three things for you:
/// 1. Converts the raw pointer to a reference.
/// 2. Converts the pointers into slices correctly.
/// 3. Treats possible panics, converting them to errors. Panics are always unwanted, but
///    panicking through an FFI boundary is UB. Therefore, this treatment is always
///    necessary.
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
///     safe_method!(something_safe)  // can only call from inside an impl block!
///                                   // This is for type safety reasons
/// }
///
/// ```
#[macro_export]
macro_rules! safe_method {
    ($safe_interface:ident) => {{
        pub unsafe extern "C" fn safe_interface(
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

                    let resource = &*(resource_ptr as *const _);

                    $safe_interface(
                        resource,
                        $crate::resource::Input::new(input_ptr, input_slots as usize),
                        $crate::resource::OutputBuilder::new(output_ptr, output_slots as usize),
                    )
                }
            }) {
                Ok(Ok(())) => std::ptr::null_mut(),
                Ok(Err(err)) => crate::utils::make_safe_c_str(err).into_raw() as *mut u8,
                Err(_) => crate::utils::make_safe_c_str("method panicked. See stderr".to_string())
                    .into_raw() as *mut u8,
            }
        }

        safe_interface
    }};
}
