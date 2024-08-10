//! This crate implements the `ppca` extension for jyafn. It exposes a minimal API
//! for evaluating models in runtime.
//!
//! There are two resources declared by this extension: `PPCAModel` and `PPCAMix`. Both
//! have the same methods:
//! 
//! TODO: write doc. By now, see code.

use jyafn_ext::{Input, Method, OutputBuilder, Resource};

jyafn_ext::extension! {
    PPCAModel,
    PPCAMix
}

fn size_of_model(model: &ppca::PPCAModel) -> usize {
    model.mean().len() + model.transform().len() + 1
}

struct PPCAModel {
    model: ppca::PPCAModel,
}

impl Resource for PPCAModel {
    fn from_bytes(bytes: &[u8]) -> Result<Self, impl ToString> {
        let model: ppca::PPCAModel = bincode::deserialize(bytes)?;
        Ok::<_, bincode::Error>(PPCAModel { model })
    }

    fn dump(&self) -> Result<Vec<u8>, impl ToString> {
        bincode::serialize(&self.model)
    }

    /// We cannot know the size of this model... ;(
    fn size(&self) -> usize {
        size_of_model(&self.model)
    }

    fn get_method(&self, method: &str) -> Option<Method> {
        jyafn_ext::declare_methods! {
            match method:
                llk(sample: [scalar; self.model.output_size()]) -> scalar;
                extrapolate(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                extrapolated_covariance_diagonal(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                smooth(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                smoothed_covariance_diagonal(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                components(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.state_size()];
                component_covariance(sample: [scalar; self.model.output_size()])
                    -> [[scalar; self.model.state_size()]; self.model.state_size()];
        }
    }
}

impl PPCAModel {
    fn llk(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let llk = self.model.llk_one(&sample);
        output_builder.push_f64(llk);
        Ok(())
    }

    jyafn_ext::method!(llk);

    fn extrapolate(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let extrapolated = self.model.extrapolate_one(&sample);
        output_builder.copy_from_f64(&extrapolated.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(extrapolate);

    fn extrapolated_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);
        output_builder.copy_from_f64(
            &inferred
                .extrapolated_covariance_diagonal(&self.model, &sample)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(extrapolated_covariance_diagonal);

    fn smooth(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let smoothed = self.model.smooth_one(&sample);
        output_builder.copy_from_f64(&smoothed.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(smooth);

    fn smoothed_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);
        output_builder.copy_from_f64(
            &inferred
                .smoothed_covariance_diagonal(&self.model)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(smoothed_covariance_diagonal);

    fn components(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);
        output_builder.copy_from_f64(&inferred.state().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(components);

    fn component_covariance(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);
        output_builder.copy_from_f64(&inferred.covariance().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(component_covariance);
}

struct PPCAMix {
    model: ppca::PPCAMix,
}

impl Resource for PPCAMix {
    fn from_bytes(bytes: &[u8]) -> Result<Self, impl ToString> {
        let model: ppca::PPCAMix = bincode::deserialize(bytes)?;
        Ok::<_, bincode::Error>(PPCAMix { model })
    }

    fn dump(&self) -> Result<Vec<u8>, impl ToString> {
        bincode::serialize(&self.model)
    }

    /// We cannot know the size of this model... ;(
    fn size(&self) -> usize {
        self.model.models().len()
            + self
                .model
                .models()
                .get(0)
                .map(|m| 1 + size_of_model(m))
                .unwrap_or_default()
    }

    fn get_method(&self, method: &str) -> Option<Method> {
        jyafn_ext::declare_methods! {
            match method:
                llk(sample: [scalar; self.model.output_size()]) -> scalar;
                extrapolate(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                extrapolated_covariance_diagonal(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                smooth(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                smoothed_covariance_diagonal(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
        }
    }
}

impl PPCAMix {
    fn llk(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let llk = self.model.llk_one(&sample);
        output_builder.push_f64(llk);
        Ok(())
    }

    jyafn_ext::method!(llk);

    fn extrapolate(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let extrapolated = self.model.extrapolate_one(&sample);
        output_builder.copy_from_f64(&extrapolated.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(extrapolate);

    fn extrapolated_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);
        output_builder.copy_from_f64(
            &inferred
                .extrapolated_covariance_diagonal(&self.model, &sample)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(extrapolated_covariance_diagonal);

    fn smooth(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let smoothed = self.model.smooth_one(&sample);
        output_builder.copy_from_f64(&smoothed.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(smooth);

    fn smoothed_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);
        output_builder.copy_from_f64(
            &inferred
                .smoothed_covariance_diagonal(&self.model)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(smoothed_covariance_diagonal);
}
