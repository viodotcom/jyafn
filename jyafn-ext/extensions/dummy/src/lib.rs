//! This crate implements the `dummy` extension for jyafn. This extension is intended for
//! testing and debugging purposes.
//!
//! The only resource declared by this extension is the `Dummy` resource, with three methods:
//! ```
//! // Gets the divison of `x` by the number supplied in the resource creation.
//! get(x: scalar) -> scalar;
//! // Always errors.
//! err(x: scalar) -> scalar;
//! // Always panics.
//! // NOTE: the panic is caught by the macros in `jyafn-ext` and transformed into an
//! // error. Panics can never propagate to jyafn code, ever!
//! panic(x: scalar) -> scalar;
//! ```

use jyafn_ext::{Method, Resource};

jyafn_ext::extension! {
    Dummy
}

#[derive(Debug)]
struct Dummy {
    number: f64,
}

impl Dummy {
    #[inline]
    fn do_get(&self, x: f64) -> f64 {
        x / self.number
    }

    fn get(
        &self,
        input: jyafn_ext::Input,
        mut output: jyafn_ext::OutputBuilder,
    ) -> Result<(), String> {
        output.push_f64(self.do_get(input.get_f64(0)));
        Ok(())
    }

    jyafn_ext::method!(get);

    fn err(&self, _: jyafn_ext::Input, _: jyafn_ext::OutputBuilder) -> Result<(), String> {
        Err("oops! wrooong!!".to_string())
    }

    jyafn_ext::method!(err);

    fn panic(&self, _: jyafn_ext::Input, _: jyafn_ext::OutputBuilder) -> Result<(), String> {
        panic!("g-g-g-g-ghost!")
    }

    jyafn_ext::method!(panic);
}

impl Resource for Dummy {
    fn size(&self) -> usize {
        0
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, impl ToString> {
        String::from_utf8_lossy(bytes)
            .parse::<f64>()
            .map(|number| Dummy { number })
    }

    fn dump(&self) -> Result<Vec<u8>, impl ToString> {
        Ok::<_, String>(self.number.to_string().into())
    }

    fn get_method(&self, method: &str) -> Option<Method> {
        jyafn_ext::declare_methods! {
            match method:
                get(x: scalar) -> scalar;
                err(x: scalar) -> scalar;
                panic(x: scalar) -> scalar;
        }
    }
}
