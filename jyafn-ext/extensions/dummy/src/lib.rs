use jyafn_ext::Resource;

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
        Ok(output.push_f64(self.do_get(input.get_f64(0))))
    }

    jyafn_ext::method!(get);

    fn err(
        &self,
        _: jyafn_ext::Input,
        _: jyafn_ext::OutputBuilder,
    ) -> Result<(), String> {
        Err("oops! wrooong!!".to_string())
    }

    jyafn_ext::method!(err);

    fn panic(
        &self,
        _: jyafn_ext::Input,
        _: jyafn_ext::OutputBuilder,
    ) -> Result<(), String> {
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

    jyafn_ext::declare_methods! {
        get(x: scalar) -> scalar
        err(x: scalar) -> scalar
        panic(x: scalar) -> scalar
    }
}
