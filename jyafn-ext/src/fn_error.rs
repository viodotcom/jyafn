/// The error type returned from the compiled function. If you need to create a new error
/// from your code, use `String::into`.
pub struct FnError(Option<String>);

impl From<String> for FnError {
    fn from(s: String) -> FnError {
        FnError(Some(s))
    }
}

impl FnError {
    /// Takes the underlying error message from this error. Calling this method more than
    /// once will result in a panic.
    pub fn take(&mut self) -> String {
        self.0.take().expect("can only call take once")
    }
}
