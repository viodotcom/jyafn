//! This crate implements the `ppca` extension for jyafn. It exposes a minimal API
//! for evaluating models in runtime.
//!
//! There are two resources declared by this extension: `PPCAModel` and `PPCAMix`. Both
//! have the same methods:
//!
//! TODO: write doc. By now, see code (Ctrl+F `declare_methods!`)

use jyafn_ext::{Input, InputReader, Method, OutputBuilder, Resource};
use nalgebra::{DMatrix, DVector};

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
                infer(sample: [scalar; self.model.output_size()]) -> {
                    state: [scalar; self.model.state_size()],
                    covariance: [[scalar; self.model.state_size()]; self.model.state_size()]
                };
                extrapolate(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                extrapolated(
                    sample: [scalar; self.model.output_size()],
                    state: [scalar; self.model.state_size()],
                    covariance: [[scalar; self.model.state_size()]; self.model.state_size()]
                ) -> [scalar; self.model.output_size()];
                extrapolated_covariance_diagonal(
                    sample: [scalar; self.model.output_size()],
                    state: [scalar; self.model.state_size()],
                    covariance: [[scalar; self.model.state_size()]; self.model.state_size()]
                ) -> [scalar; self.model.output_size()];
                smooth(sample: [scalar; self.model.output_size()])
                    -> [scalar; self.model.output_size()];
                smoothed(
                    state: [scalar; self.model.state_size()],
                    covariance: [[scalar; self.model.state_size()]; self.model.state_size()]
                ) -> [scalar; self.model.output_size()];
                smoothed_covariance_diagonal(
                    state: [scalar; self.model.state_size()],
                    covariance: [[scalar; self.model.state_size()]; self.model.state_size()]
                ) -> [scalar; self.model.output_size()];
        }
    }
}

impl PPCAModel {
    fn read_inferred(&self, reader: &mut InputReader) -> ppca::InferredMasked {
        let state = DVector::from(reader.read_n_f64(self.model.state_size()));
        let covariance = DMatrix::from_row_iterator(
            self.model.state_size(),
            self.model.state_size(),
            reader.read_n_f64(self.model.state_size() * self.model.state_size()),
        );

        self.model.inferred_one(state, covariance)
    }

    fn read_sample(&self, reader: &mut InputReader) -> ppca::MaskedSample {
        ppca::MaskedSample::mask_non_finite(reader.read_n_f64(self.model.output_size()).into())
    }
}

impl PPCAModel {
    fn llk(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let llk = self.model.llk_one(&sample);
        output_builder.push_f64(llk);
        Ok(())
    }

    jyafn_ext::method!(llk);

    fn extrapolate(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let extrapolated = self.model.extrapolate_one(&sample);
        output_builder.copy_from_f64(extrapolated.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(extrapolate);

    fn extrapolated(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(inferred.extrapolated(&self.model, &sample).data.as_vec());
        Ok(())
    }
    jyafn_ext::method!(extrapolated);

    fn extrapolated_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(
            inferred
                .extrapolated_covariance_diagonal(&self.model, &sample)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(extrapolated_covariance_diagonal);

    fn smooth(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let smoothed = self.model.smooth_one(&sample);
        output_builder.copy_from_f64(smoothed.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(smooth);

    fn smoothed(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(inferred.smoothed(&self.model).data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(smoothed);

    fn smoothed_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(
            inferred
                .smoothed_covariance_diagonal(&self.model)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(smoothed_covariance_diagonal);

    fn infer(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);
        output_builder.copy_from_f64(inferred.state().data.as_vec());
        output_builder.copy_from_f64(inferred.covariance().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(infer);
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
                .first()
                .map(|m| 1 + size_of_model(m))
                .unwrap_or_default()
    }

    fn get_method(&self, method: &str) -> Option<Method> {
        if let Some(state_size) = self.maybe_state_size() {
            jyafn_ext::declare_methods! {
                match method:
                    llk(sample: [scalar; self.model.output_size()]) -> scalar;
                    infer(sample: [scalar; self.model.output_size()]) -> {
                        log_posterior: [scalar; self.model.models().len()],
                        state: [[scalar; state_size]; self.model.models().len()],
                        covariance: [[[scalar; state_size]; state_size]; self.model.models().len()]
                    };
                    extrapolate(sample: [scalar; self.model.output_size()])
                        -> [scalar; self.model.output_size()];
                    extrapolated(
                        sample: [scalar; self.model.output_size()],
                        log_posterior: [scalar; self.model.models().len()],
                        state: [[scalar; state_size]; self.model.models().len()],
                        covariance: [[[scalar; state_size]; state_size]; self.model.models().len()]
                    ) -> [scalar; self.model.output_size()];
                    extrapolated_covariance_diagonal(
                        sample: [scalar; self.model.output_size()],
                        log_posterior: [scalar; self.model.models().len()],
                        state: [[scalar; state_size]; self.model.models().len()],
                        covariance: [[[scalar; state_size]; state_size]; self.model.models().len()]
                    ) -> [scalar; self.model.output_size()];
                    smooth(sample: [scalar; self.model.output_size()])
                        -> [scalar; self.model.output_size()];
                    smoothed(
                        sample: [scalar; self.model.output_size()],
                        log_posterior: [scalar; self.model.models().len()],
                        state: [[scalar; state_size]; self.model.models().len()],
                        covariance: [[[scalar; state_size]; state_size]; self.model.models().len()]
                    ) -> [scalar; self.model.output_size()];
                    smoothed_covariance_diagonal(
                        sample: [scalar; self.model.output_size()],
                        log_posterior: [scalar; self.model.models().len()],
                        state: [[scalar; state_size]; self.model.models().len()],
                        covariance: [[[scalar; state_size]; state_size]; self.model.models().len()]
                    ) -> [scalar; self.model.output_size()];
            }
        } else {
            jyafn_ext::declare_methods! {
                match method:
                    llk(sample: [scalar; self.model.output_size()]) -> scalar;
                    extrapolate(sample: [scalar; self.model.output_size()])
                        -> [scalar; self.model.output_size()];
                    smooth(sample: [scalar; self.model.output_size()])
                        -> [scalar; self.model.output_size()];
            }
        }
    }
}

impl PPCAMix {
    /// Decide on a state size for this mixture model. Mixture models _may_ have
    /// differently sized sub-states. In this case, some methods will not be available.
    fn maybe_state_size(&self) -> Option<usize> {
        self.model.models().first().and_then(|m| {
            let state_size = m.state_size();
            let all_same = self
                .model
                .models()
                .iter()
                .all(|m| m.state_size() == state_size);
            if all_same {
                Some(state_size)
            } else {
                None
            }
        })
    }

    fn read_sample(&self, reader: &mut InputReader) -> ppca::MaskedSample {
        ppca::MaskedSample::mask_non_finite(reader.read_n_f64(self.model.output_size()).into())
    }

    fn read_inferred(&self, reader: &mut InputReader) -> ppca::InferredMaskedMix {
        let log_posteriors = reader.read_n_f64(self.model.models().len());
        let state_size = self.maybe_state_size().unwrap();
        let sub_states = self
            .model
            .models()
            .iter()
            .map(|m| {
                let state = DVector::from(reader.read_n_f64(state_size));
                let covariance = DMatrix::from_row_iterator(
                    state_size,
                    state_size,
                    reader.read_n_f64(state_size * state_size),
                );
                m.inferred_one(state, covariance)
            })
            .collect();
        self.model.inferred_one(log_posteriors.into(), sub_states)
    }
}

impl PPCAMix {
    fn llk(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let llk = self.model.llk_one(&sample);
        output_builder.push_f64(llk);
        Ok(())
    }

    jyafn_ext::method!(llk);

    fn extrapolate(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let extrapolated = self.model.extrapolate_one(&sample);
        output_builder.copy_from_f64(extrapolated.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(extrapolate);

    fn extrapolated(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(inferred.extrapolated(&self.model, &sample).data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(extrapolated);

    fn extrapolated_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(
            inferred
                .extrapolated_covariance_diagonal(&self.model, &sample)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(extrapolated_covariance_diagonal);

    fn smooth(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let smoothed = self.model.smooth_one(&sample);
        output_builder.copy_from_f64(smoothed.data_vector().data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(smooth);

    fn smoothed(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(inferred.extrapolated(&self.model, &sample).data.as_vec());
        Ok(())
    }

    jyafn_ext::method!(smoothed);

    fn smoothed_covariance_diagonal(
        &self,
        input: Input,
        mut output_builder: OutputBuilder,
    ) -> Result<(), String> {
        let mut reader = InputReader::new(input);
        let sample = self.read_sample(&mut reader);
        let inferred = self.read_inferred(&mut reader);
        output_builder.copy_from_f64(
            inferred
                .extrapolated_covariance_diagonal(&self.model, &sample)
                .data
                .as_vec(),
        );
        Ok(())
    }

    jyafn_ext::method!(smoothed_covariance_diagonal);

    fn infer(&self, input: Input, mut output_builder: OutputBuilder) -> Result<(), String> {
        let sample = ppca::MaskedSample::mask_non_finite(input.as_f64_slice().to_owned().into());
        let inferred = self.model.infer_one(&sample);

        output_builder.copy_from_f64(inferred.log_posterior().data.as_vec());

        for inferred in inferred.sub_states() {
            output_builder.copy_from_f64(inferred.state().data.as_vec());
        }

        for inferred in inferred.sub_states() {
            output_builder.copy_from_f64(inferred.covariance().data.as_vec());
        }

        Ok(())
    }

    jyafn_ext::method!(infer);
}
