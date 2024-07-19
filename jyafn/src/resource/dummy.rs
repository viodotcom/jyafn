#![cfg(feature = "lightgbm")]

use serde_derive::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;

use crate::layout::{Layout, Struct};
use crate::Error;

use super::{Input, OutputBuilder, Resource, ResourceMethod, ResourceType};

#[derive(Debug, Serialize, Deserialize)]
struct Dummy;

#[typetag::serde]
impl ResourceType for Dummy {
    fn from_bytes(&self, bytes: &[u8]) -> Result<Pin<Box<dyn Resource>>, Error> {
        Ok(Box::pin(DummyResource {
            number_to_divide: String::from_utf8_lossy(bytes)
                .parse::<f64>()
                .map_err(|err| err.to_string())?,
        }))
    }

    fn get_method(&self, method: &str) -> Option<ResourceMethod> {
        match method {
            "get" => Some(ResourceMethod {
                fn_ptr: crate::safe_method!(dummy_get),
                input_layout: Struct(vec![("x".to_string(), Layout::Scalar)]),
                output_layout: Layout::Scalar,
            }),
            "panic" => Some(ResourceMethod {
                fn_ptr: crate::safe_method!(dummy_panic),
                input_layout: Struct(vec![]),
                output_layout: Layout::Scalar,
            }),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct DummyResource {
    number_to_divide: f64,
}

impl Resource for DummyResource {
    fn r#type(&self) -> Arc<dyn ResourceType> {
        Arc::new(Dummy)
    }

    fn dump(&self) -> Result<Vec<u8>, Error> {
        Ok(self.number_to_divide.to_string().into())
    }

    fn size(&self) -> usize {
        0
    }
}

fn dummy_get(
    resource: &DummyResource,
    input: Input,
    mut output_builder: OutputBuilder,
) -> Result<(), String> {
    let result = input.get_f64(0) / resource.number_to_divide;
    if !result.is_finite() {
        return Err("result was not finite".to_string());
    }
    output_builder.push_f64(result);
    Ok(())
}

fn dummy_panic(
    _resource: &DummyResource,
    _input: Input,
    _output_builder: OutputBuilder,
) -> Result<(), String> {
    panic!("panic!")
}
