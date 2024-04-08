use std::error::Error as StdError;

use crate::Error;

use super::symbols::Sym;
use super::{Layout, Visitor};

pub trait Encode {
    type Err: 'static + StdError + Send;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), Self::Err>;
}

impl Encode for f64 {
    type Err = Error;
    fn visit(&self, layout: &Layout, _: &mut dyn Sym, visitor: &mut Visitor) -> Result<(), Error> {
        match layout {
            Layout::Scalar => visitor.push(*self),
            _ => return Err("expected scalar".to_string().into()),
        }

        Ok(())
    }
}

impl Encode for serde_json::Value {
    type Err = Error;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), Error> {
        match (self, layout) {
            (Self::Null, Layout::Unit) => {}
            (Self::Bool(b), Layout::Bool) => {
                visitor.push_int(*b as i64);
            }
            (Self::Number(num), Layout::Scalar) => {
                if let Some(int) = num.as_i64() {
                    visitor.push(int as f64)
                } else if let Some(uint) = num.as_u64() {
                    visitor.push(uint as f64)
                } else {
                    visitor.push(
                        num.as_f64()
                            .ok_or_else(|| format!("{num} cannot be represented as float64"))?,
                    )
                }
            }
            (Self::String(e), Layout::Symbol) => {
                let Some(index) = symbols.find(&e) else {
                    return Err(format!("symbol {e:?} not found").into());
                };
                visitor.push_int(index as i64);
            }
            (Self::Array(array), Layout::List(element, size)) => {
                if array.len() != *size {
                    return Err(format!(
                        "expected array of size {size}, got array of size {}",
                        array.len()
                    )
                    .into());
                }
                for item in array {
                    item.visit(element, symbols, visitor)?;
                }
            }
            (Self::Object(map), Layout::Struct(fields)) => {
                for (name, field) in &fields.0 {
                    let Some(value) = map.get(name) else {
                        return Err(format!("missing field {name:?} in {self:?}").into());
                    };
                    value.visit(field, symbols, visitor)?;
                }
            }
            _ => return Err(format!("incompatible layout {layout} for {self:?}").into()),
        }

        Ok(())
    }
}
