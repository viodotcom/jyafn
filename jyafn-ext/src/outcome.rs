use std::ffi::CString;

/// A type-erased result-like type to transport information on fallible operations safely
/// across the FFI boundary.
pub enum Outcome {
    /// Operation is successful and the result is stored _in the heap_ (i.e., it's a
    /// `Box` in diguise) at the supplied location.
    Ok(*mut ()),
    /// Operation is unsuccessful and an error message is supplied as a C-style string.
    Err(CString),
}

impl<T, E> From<Result<T, E>> for Outcome
where
    E: ToString,
{
    fn from(result: Result<T, E>) -> Outcome {
        match result {
            Ok(b) => Outcome::Ok(Box::leak(Box::new(b)) as *mut T as *mut ()),
            Err(e) => Outcome::Err(
                CString::new(e.to_string().replace('\0', "\\0")).expect("nuls have been escaped"),
            ),
        }
    }
}
