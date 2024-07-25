//! This crate implements the `lightgbm` extension for jyafn. It exposes a minimal API
//! for evaluating models in runtime.
//! 
//! The only resource declared by this extension is the `Lightgbm` resource, with three methods:
//! ```
//! // Predicts the probability of each class, given a list of feature values.
//! predict(x: [scalar; n_features]) -> [scalar; n_classes];
//! // The number of features in this model.
//! num_features() -> scalar;
//! // The number of classes in this model.
//! num_classes() -> scalar;
//! ```

use jyafn_ext::{Input, Method, OutputBuilder, Resource};
use lightgbm3::Booster;

jyafn_ext::extension! {
    Lightgbm
}

struct Lightgbm {
    booster: Booster,
}

// TODO: wise? See... https://github.com/Mottl/lightgbm3-rs/issues/6
unsafe impl Send for Lightgbm {}
unsafe impl Sync for Lightgbm {}

impl Resource for Lightgbm {
    fn from_bytes(bytes: &[u8]) -> Result<Self, impl ToString> {
        let booster = Booster::from_string(&String::from_utf8_lossy(bytes))?;
        Ok::<_, lightgbm3::Error>(Lightgbm { booster })
    }

    fn dump(&self) -> Result<Vec<u8>, impl ToString> {
        self.booster.save_string().map(Vec::from)
    }

    /// We cannot know the size of this model... ;(
    fn size(&self) -> usize {
        0
    }

    fn get_method(&self, method: &str) -> Option<Method> {
        let features = self.booster.num_features() as usize;
        let classes = self.booster.num_classes() as usize;

        jyafn_ext::declare_methods! {
            match method:
                predict(x: [scalar; features]) -> [scalar; classes];
                num_features() -> scalar;
                num_classes() -> scalar;
        }
    }
}

impl Lightgbm {
    fn predict(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        match self
            .booster
            .predict(input.as_f64_slice(), self.booster.num_features(), true)
        {
            Ok(classes) => {
                output_builder.copy_from_f64(&classes);
                Ok(())
            }
            Err(err) => Err(err.to_string()),
        }
    }

    jyafn_ext::method!(predict);

    fn num_features(&self, _: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        Ok(output_builder.push_f64(self.booster.num_features() as f64))
    }

    jyafn_ext::method!(num_features);

    fn num_classes(&self, _: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        Ok(output_builder.push_f64(self.booster.num_classes() as f64))
    }

    jyafn_ext::method!(num_classes);
}
