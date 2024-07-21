use serde_derive::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::pin::Pin;
use std::sync::Arc;

use crate::extension::{Dumped, Extension, ExternalMethod, RawResource, ResourceSymbols};
use crate::Error;

use super::{Resource, ResourceMethod, ResourceType};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct External {
    extension: String,
    resource: String,
}

impl External {
    fn load_extension(&self) -> Result<(), Error> {
        crate::extension::load(&self.extension)?;
        let extension = crate::extension::get(&self.extension);

        if extension.get_resource(&self.resource).is_none() {
            return Err(format!(
                "extension {} has no resource type named {}",
                self.extension, self.resource
            )
            .into());
        }

        Ok(())
    }

    fn extension(&self) -> Arc<Extension> {
        crate::extension::get(&self.extension)
    }

    fn resource(&self) -> ResourceSymbols {
        self.extension()
            .get_resource(&self.resource)
            .expect("resource not found in extension")
    }
}

#[typetag::serde]
impl ResourceType for External {
    fn from_bytes(&self, bytes: &[u8]) -> Result<Pin<Box<dyn Resource>>, Error> {
        // The _only_ way to create an `ExternalResource` is through this function. This
        // guarantees that the extension was initalized and that the resource exists.
        self.load_extension()?;

        let extension = self.extension();
        let resource = self.resource();
        let outcome = unsafe {
            // Safety: extension is correctly implemented.
            (resource.fn_from_bytes)(bytes.as_ptr(), bytes.len())
        };
        let raw_ptr = unsafe {
            // Safety: the outcome wasjust generated by the extension and never used.
            extension.outcome_to_result(outcome)?
        };

        if raw_ptr == std::ptr::null_mut() {
            return Err(format!("loaded resource for {self:?} from bytes was null").into());
        }

        Ok(Box::pin(ExternalResource {
            r#type: self.clone(),
            ptr: RawResource(raw_ptr),
        }))
    }
}

#[derive(Debug)]
struct ExternalResource {
    r#type: External,
    ptr: RawResource,
}

// Safety: all resources must be thread-safe.
unsafe impl Send for ExternalResource {}
// Safety: all resources must be thread-safe.
unsafe impl Sync for ExternalResource {}

impl Drop for ExternalResource {
    fn drop(&mut self) {
        // This cannot panic, ever! Therefore, we prefer to leak, if necessary.
        // Probably, the `else` is never reachable, but... better safe than sorry.
        if let Some(extension) = crate::extension::get_opt(&self.r#type.extension) {
            if let Some(resource) = extension.get_resource(&self.r#type.resource) {
                unsafe {
                    // Safety: extension is correctly implemented.
                    (resource.fn_drop)(self.ptr);
                }
            }
        }
    }
}

impl Resource for ExternalResource {
    fn r#type(&self) -> Arc<dyn ResourceType> {
        Arc::new(self.r#type.clone())
    }

    fn dump(&self) -> Result<Vec<u8>, Error> {
        let resource = self.r#type.resource();
        let extension = self.r#type.extension();
        unsafe {
            // Safety: extension is correctly implemented.
            let maybe_outcome = (resource.fn_dump)(self.ptr);
            if maybe_outcome.0 == std::ptr::null_mut() {
                return Err(format!("dumped resource for {:?} was null", self.r#type).into());
            }
            extension.dumped_to_vec(Dumped(extension.outcome_to_result(maybe_outcome)?))
        }
    }

    fn size(&self) -> usize {
        unsafe {
            // Safety: extension is correctly implemented.
            (self.r#type.resource().fn_size)(self.ptr)
        }
    }

    fn get_raw_ptr(&self) -> *const () {
        self.ptr.0 as *const ()
    }

    fn get_method(&self, method: &str) -> Option<ResourceMethod> {
        let c_method = CString::new(method.as_bytes()).expect("method cannot contain nul bytes");
        let resource = self.r#type.resource();

        let external_method = unsafe {
            // Safety: extension is correctly implemented.
            let maybe_method = (resource.fn_get_method_def)(self.ptr, c_method.as_ptr());
            if maybe_method == std::ptr::null_mut() {
                return None;
            }
            scopeguard::defer! {
                (resource.fn_drop_method_def)(maybe_method)
            }

            serde_json::from_slice::<ExternalMethod>(CStr::from_ptr(maybe_method).to_bytes())
                .expect("badly formed json from fn_get_method call")
        };

        Some(ResourceMethod {
            fn_ptr: unsafe {
                // Safety: this should have been a valid address in the extension side.
                std::mem::transmute(external_method.fn_ptr)
            },
            input_layout: external_method.input_layout,
            output_layout: external_method.output_layout,
        })
    }
}