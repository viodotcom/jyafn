#![cfg(feature = "lightgbm")]

use lightgbm3::Booster;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use std::pin::Pin;
use std::sync::Arc;

use crate::layout::{Layout, Struct};
use crate::Error;

use super::{Input, OutputBuilder, Resource, ResourceMethod, ResourceType};

#[derive(Debug, Serialize, Deserialize)]
struct Lightgbm {
    n_features: i32,
    n_classes: i32,
}

#[typetag::serde]
impl ResourceType for Lightgbm {
    fn from_bytes(&self, bytes: &[u8]) -> Result<Pin<Box<dyn Resource>>, Error> {
        let booster =
            Booster::from_string(&String::from_utf8_lossy(bytes)).map_err(|err| err.to_string())?;

        if booster.num_features() != self.n_features {
            return Err(format!(
                "Expected {} features, got {}",
                self.n_features,
                booster.num_features(),
            )
            .into());
        }

        if booster.num_classes() != self.n_classes {
            return Err(format!(
                "Expected {} classes, got {}",
                self.n_classes,
                booster.num_classes(),
            )
            .into());
        }

        Ok(Box::pin(LightgbmResource { booster }))
    }

    fn get_method(&self, method: &str) -> Option<ResourceMethod> {
        match method {
            "predict" => Some(ResourceMethod {
                fn_ptr: crate::safe_method!(predict_method),
                input_layout: Struct(vec![(
                    "x".to_string(),
                    Layout::List(Box::new(Layout::Scalar), self.n_features as usize),
                )]),
                output_layout: Layout::List(Box::new(Layout::Scalar), self.n_classes as usize),
            }),
            _ => None,
        }
    }
}

struct LightgbmResource {
    booster: Booster,
}

// TODO: wise?
unsafe impl Send for LightgbmResource {}
unsafe impl Sync for LightgbmResource {}

impl Debug for LightgbmResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(LightgbmResource))
    }
}

impl Resource for LightgbmResource {
    fn r#type(&self) -> Arc<dyn ResourceType> {
        Arc::new(Lightgbm {
            n_features: self.booster.num_features(),
            n_classes: self.booster.num_classes(),
        })
    }

    fn dump(&self) -> Result<Vec<u8>, Error> {
        Ok(self
            .booster
            .save_string()
            .map_err(|err| err.to_string())?
            .into())
    }

    /// We cannot know the size of this model.
    fn size(&self) -> usize {
        0
    }
}

fn predict_method(
    resource: &LightgbmResource,
    input: Input,
    mut output_builder: OutputBuilder,
) -> Result<(), String> {
    match resource
        .booster
        .predict(input.as_f64_slice(), resource.booster.num_features(), true)
    {
        Ok(classes) => {
            output_builder.copy_from_f64(&classes);
            Ok(())
        }
        Err(err) => Err(err.to_string()),
    }
}
