use serde_derive::{Deserialize, Serialize};

use super::{Layout, Struct};

/// A `Resource` is an amount of data associated with "methods", much like an object in
/// OO languages, but simpler. Specifically, resources shoud _not_ (ever!) support
/// mutation. Resources are immutable pices of data.
///
/// # Note
///
/// This is a more convenient version of the `Resource` trait used in the `jyafn` crate,
/// at the cost of object safety, which we don't need for our usecase.
pub trait Resource: 'static + Sized + Send + Sync {
    /// Creates a resource out of binary data.
    fn from_bytes(bytes: &[u8]) -> Result<Self, impl ToString>;
    /// Dumps this resource as binary data.
    fn dump(&self) -> Result<Vec<u8>, impl ToString>;
    /// The ammount of heap used by this storage.
    fn size(&self) -> usize;
    /// Gets information on a method name for this resource, if it exists.
    fn get_method(&self, name: &str) -> Option<Method>;
}

/// A description on the method signature, to guide jyafn to generate the correct method
/// call on this resource.
#[derive(Debug, Serialize, Deserialize)]
pub struct Method {
    /// The function pointer to be used in the jyafn code.
    pub fn_ptr: usize,
    /// The layout of the input parameters.
    pub input_layout: Struct,
    /// The layout of the output parameters.
    pub output_layout: Layout,
}
