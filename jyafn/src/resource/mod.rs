pub mod lightgbm;

use downcast_rs::{impl_downcast, Downcast};
use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::io::Read;
use std::mem::MaybeUninit;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::pin::Pin;
use std::sync::Arc;
use zip::read::ZipFile;

use crate::layout::{Layout, Struct};
use crate::{Error, FnError};

pub type RawResourceMethod =
    unsafe extern "C" fn(*const ResourceContainer, *const u8, u64, *mut u8, u64) -> *mut FnError;

/// A safe convenience macro for method call. This macro does three things for you:
/// 1. Converts the raw pointer to a reference.
/// 2. Converts the pointers into slices correctly.
/// 3. Treats possible panics, converting them to errors. Panics are always unwanted, but
/// panicking through an FFI boundary is UB. Therefore, this treatment is always necessary.
///
/// # Usage
///
/// ```
/// fn safe(
///     container: &ResourceContainer, input: Input, output: OutputBuilder,
/// ) -> Result<(), &'static CStr> {
///     // ...
///     todo!()
/// }
///
/// safe_method!(safe)
/// ```
#[macro_export]
macro_rules! safe_method {
    ($safe_interface:ident) => {{
        pub unsafe extern "C" fn safe_interface(
            container_ptr: *const ResourceContainer,
            input_ptr: *const u8,
            input_slots: u64,
            output_ptr: *mut u8,
            output_slots: u64,
        ) -> *mut $crate::FnError {
            match std::panic::catch_unwind(|| {
                let container: &$crate::resource::ResourceContainer = unsafe {
                    // Safety: this pointer came from jyafn code.
                    &*container_ptr
                };
                let input = unsafe {
                    // Safety: this pointer and length came from jyafn code.
                    $crate::resource::Input::new(input_ptr, input_slots as usize)
                };
                let output = unsafe {
                    // Safety: this pointer and length came from jyafn code.
                    $crate::resource::OutputBuilder::new(output_ptr, output_slots as usize)
                };

                $safe_interface(container, input, output)
            }) {
                Ok(Ok(())) => std::ptr::null_mut(),
                Ok(Err(err)) => {
                    let boxed = Box::new(err.to_string().into());
                    Box::leak(boxed)
                }
                // DON'T forget the nul character when working with bytes directly!
                Err(_) => {
                    let boxed = Box::new("method panicked. See stdout".to_string().into());
                    Box::leak(boxed)
                }
            }
        }

        safe_interface
    }};
}

pub struct ResourceMethod {
    pub(crate) fn_ptr: RawResourceMethod,
    pub(crate) input_layout: Struct,
    pub(crate) output_layout: Layout,
}

/// A `ResourceType` creates resources of a givnen type. Think of this as the "class
/// object" of resources.
#[typetag::serde(tag = "type")]
pub trait ResourceType:
    std::fmt::Debug + Send + Sync + UnwindSafe + RefUnwindSafe + Downcast
{
    /// Creates a resource out of binary data.
    fn from_bytes(&self, bytes: &[u8]) -> Result<Pin<Box<dyn Resource>>, Error>;
    /// Gets information on a method name for this resource, if it exists.
    fn get_method(&self, method: &str) -> Option<ResourceMethod>;

    /// Reads a resource from a zip file entry.
    ///
    /// Override this method if you know a more efficient of loading the resouce other
    /// than reading the file to a buffer and then parsing the resulting buffer.
    fn read(&self, mut f: ZipFile<'_>) -> Result<Pin<Box<dyn Resource>>, Error> {
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        self.from_bytes(&buffer)
    }
}

impl_downcast!(ResourceType);

/// A `Resource` is an amount of data associated with "methods", much like an object in
/// OO languages, but simpler. Specifically, resources shoud _not_ (ever!) support
/// mutation. Resources are immutable pices of data.
pub trait Resource:
    'static + std::fmt::Debug + Send + Sync + UnwindSafe + RefUnwindSafe + Downcast
{
    /// Returns the type of this resource. This has to be the same value that, if applied
    /// to the output of `Resource:dump`, will again yield this exact resource.
    fn r#type(&self) -> Arc<dyn ResourceType>;
    /// Dumps this resource as binary data.
    fn dump(&self) -> Result<Vec<u8>, Error>;
    /// The ammount of heap used by this storage.
    fn size(&self) -> usize;
}

impl_downcast!(Resource);

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

    /// Gets a information on a method for the containted resource, if it exists.
    pub fn get_method(&self, method: &str) -> Option<ResourceMethod> {
        self.resource_type.get_method(method)
    }

    /// Gets the resource as the supplied type. This method panics if the container is
    /// not initialized or if the contained resource is not of type `R`.
    pub fn get_resource<R: Resource>(&self) -> &R {
        self.resource
            .as_ref()
            .expect("resource not initialized")
            .downcast_ref::<R>()
            .expect("cannot downcast resource to the specified type")
    }

    /// Gets the resource type as the supplied type. This method panics if the container
    /// is not initialized or if the contained resource type is not of type `T`.
    pub fn get_resource_type<T: ResourceType>(&self) -> &T {
        self.resource_type
            .downcast_ref::<T>()
            .expect("cannot downcast resource to the specified type")
    }

    /// Executes a function over the resource.  This method panics if the container is
    /// not initialized or if either the contained resource type is not of type `T` or
    /// the resource is not of type `R`.
    pub fn with_resource<F, T, R, U>(&self, f: F) -> U
    where
        T: ResourceType,
        R: Resource,
        F: FnOnce(&T, &R) -> U,
    {
        f(self.get_resource_type(), self.get_resource())
    }
}

/// A convenience wrapper over the input data pointer, given the information on its size.
#[repr(transparent)]
pub struct Input<'a>(&'a [u64]);

impl<'a> Input<'a> {
    /// Creates a new input.
    ///
    /// # Safety:
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
        unsafe {
            // Safety: f64 and u64 have the same size and offset.
            std::mem::transmute(self.0)
        }
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

impl<'a> OutputBuilder<'a> {
    /// Creates a new input.
    ///
    /// # Safety:
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
