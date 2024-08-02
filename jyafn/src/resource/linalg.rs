//! Linalg resources for jyafn.
//!
//! It turns out that we _could_ implement the more complext linalg operations in jyafn
//! directly. However, it's not because you can do it that you _should_ do it. Some
//! operations are quite intrincate to replicate and a good pure Python implementation
//! that supports the way jyafn does branching is lacking. Also, the graph building model
//! used by jyafn can lead to code that takes a long while to compile.
//!

use faer::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{layout, r#struct, safe_method};

use super::{Input, OutputBuilder, Resource, ResourceMethod, ResourceType};

#[derive(Debug, Serialize, Deserialize)]
struct SquareMatrix;

#[typetag::serde]
impl ResourceType for SquareMatrix {
    fn from_bytes(&self, bytes: &[u8]) -> Result<std::pin::Pin<Box<dyn Resource>>, crate::Error> {
        Ok(Box::pin(SquareMatrixResource {
            shape: String::from_utf8_lossy(bytes)
                .parse::<usize>()
                .map_err(|err| err.to_string())?,
        }))
    }
}

#[derive(Debug)]
struct SquareMatrixResource {
    shape: usize,
}

impl Resource for SquareMatrixResource {
    fn r#type(&self) -> Arc<dyn ResourceType> {
        Arc::new(SquareMatrix)
    }

    fn dump(&self) -> Result<Vec<u8>, crate::Error> {
        Ok(self.shape.to_string().into_bytes())
    }

    fn size(&self) -> usize {
        0
    }

    fn get_method(&self, method: &str) -> Option<super::ResourceMethod> {
        Some(match method {
            "det" => ResourceMethod {
                input_layout: r#struct!(
                    a: [[scalar; self.shape]; self.shape]
                ),
                output_layout: layout!(scalar),
                fn_ptr: safe_method!(matix_det),
            },
            "inv" => ResourceMethod {
                input_layout: r#struct!(
                    a: [[scalar; self.shape]; self.shape]
                ),
                output_layout: layout!([[scalar; self.shape]; self.shape]),
                fn_ptr: safe_method!(matrix_inv),
            },
            "solve" => ResourceMethod {
                input_layout: r#struct!(
                    a: [[scalar; self.shape]; self.shape],
                    v: [scalar; self.shape]
                ),
                output_layout: layout!([scalar; self.shape]),
                fn_ptr: safe_method!(matrix_solve),
            },
            "cholesky" => ResourceMethod {
                input_layout: r#struct!(
                    a: [[scalar; self.shape]; self.shape]
                ),
                output_layout: layout!([[scalar; self.shape]; self.shape]),
                fn_ptr: safe_method!(matrix_cholesky),
            },
            _ => return None,
        })
    }
}

fn matix_det(
    resource: &SquareMatrixResource,
    input: Input,
    mut output_builder: OutputBuilder,
) -> Result<(), String> {
    if resource.shape * resource.shape != input.len() {
        return Err(format!(
            "expected input length of {}x{} = {}, but got {}",
            resource.shape,
            resource.shape,
            resource.shape * resource.shape,
            input.len()
        ));
    }

    let mat = faer::mat::from_row_major_slice(input.as_f64_slice(), resource.shape, resource.shape);
    output_builder.push_f64(mat.determinant());

    Ok(())
}

fn matrix_inv(
    resource: &SquareMatrixResource,
    input: Input,
    mut output_builder: OutputBuilder,
) -> Result<(), String> {
    if resource.shape * resource.shape != input.len() {
        return Err(format!(
            "expected input length of {}x{} = {}, but got {}",
            resource.shape,
            resource.shape,
            resource.shape * resource.shape,
            input.len()
        ));
    }

    let mat = faer::mat::from_row_major_slice(input.as_f64_slice(), resource.shape, resource.shape);
    let inv = mat.col_piv_qr().inverse();

    for row in inv.row_iter() {
        for j in 0..resource.shape {
            output_builder.push_f64(*row.get(j));
        }
    }

    Ok(())
}

fn matrix_solve(
    resource: &SquareMatrixResource,
    input: Input,
    mut output_builder: OutputBuilder,
) -> Result<(), String> {
    if resource.shape * (resource.shape + 1) != input.len() {
        return Err(format!(
            "expected input length of {}x{} = {}, but got {}",
            resource.shape,
            resource.shape,
            resource.shape * resource.shape,
            input.len()
        ));
    }

    let input_slice = input.as_f64_slice();
    let mat = faer::mat::from_row_major_slice(
        &input_slice[..resource.shape * resource.shape],
        resource.shape,
        resource.shape,
    );
    let vec = faer::mat::from_row_major_slice(
        &input_slice[resource.shape * resource.shape..],
        resource.shape,
        1,
    );

    let solved = mat.col_piv_qr().solve(vec);

    for row in solved.row_iter() {
        output_builder.push_f64(*row.get(0));
    }

    Ok(())
}

fn matrix_cholesky(
    resource: &SquareMatrixResource,
    input: Input,
    mut output_builder: OutputBuilder,
) -> Result<(), String> {
    if resource.shape * resource.shape != input.len() {
        return Err(format!(
            "expected input length of {}x{} = {}, but got {}",
            resource.shape,
            resource.shape,
            resource.shape * resource.shape,
            input.len()
        ));
    }

    let mat = faer::mat::from_row_major_slice(input.as_f64_slice(), resource.shape, resource.shape);
    let Ok(cholesky) = mat.cholesky(faer::Side::Lower) else {
        return Err("matrix is not cholesky decomposable".to_owned());
    };

    for row in cholesky.compute_l().row_iter() {
        for j in 0..resource.shape {
            output_builder.push_f64(*row.get(j));
        }
    }

    Ok(())
}
