use std::collections::{BTreeMap, HashMap};
use std::error::Error as StdError;
use std::rc::Rc;
use std::sync::Arc;

use crate::{utils, Error};

use super::symbols::Sym;
use super::{Layout, Visitor};

/// A type that can be encoded into a jyafn context.
pub trait Encode {
    /// The errors that might arise from the encoding procedure.
    type Err: 'static + StdError + Send + Sync;
    /// Encodes this values into the provided context, given the provided layout.
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), Self::Err>;
}

impl Encode for () {
    type Err = Error;
    fn visit(&self, layout: &Layout, _: &mut dyn Sym, _: &mut Visitor) -> Result<(), Error> {
        match layout {
            Layout::Unit => {}
            _ => return Err("expected unit".to_string().into()),
        }

        Ok(())
    }
}

impl<T: Encode> Encode for &T {
    type Err = T::Err;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), T::Err> {
        (*self).visit(layout, symbols, visitor)
    }
}

macro_rules! impl_encode_container {
    ($container:ty) => {
        impl<T: Encode> Encode for $container {
            type Err = T::Err;
            fn visit(
                &self,
                layout: &Layout,
                symbols: &mut dyn Sym,
                visitor: &mut Visitor,
            ) -> Result<(), T::Err> {
                (&*self as &T).visit(layout, symbols, visitor)
            }
        }
    };
}

impl_encode_container!(&mut T);
impl_encode_container!(Box<T>);
impl_encode_container!(Rc<T>);
impl_encode_container!(Arc<T>);

macro_rules! impl_encode_scalar {
    ($scalar:ty) => {
        impl Encode for $scalar {
            type Err = Error;
            fn visit(
                &self,
                layout: &Layout,
                _: &mut dyn Sym,
                visitor: &mut Visitor,
            ) -> Result<(), Error> {
                match layout {
                    Layout::Scalar => visitor.push(*self as f64),
                    _ => return Err("expected scalar".to_string().into()),
                }

                Ok(())
            }
        }
    };
}

impl_encode_scalar!(i8);
impl_encode_scalar!(u8);
impl_encode_scalar!(i16);
impl_encode_scalar!(u16);
impl_encode_scalar!(i32);
impl_encode_scalar!(u32);
impl_encode_scalar!(i64);
impl_encode_scalar!(u64);
impl_encode_scalar!(isize);
impl_encode_scalar!(usize);
impl_encode_scalar!(f64);
impl_encode_scalar!(f32);

impl Encode for bool {
    type Err = Error;
    fn visit(&self, layout: &Layout, _: &mut dyn Sym, visitor: &mut Visitor) -> Result<(), Error> {
        match layout {
            Layout::Bool => {
                visitor.push_int(*self as i64);
            }
            _ => return Err("expected bool".to_string().into()),
        }

        Ok(())
    }
}

impl Encode for String {
    type Err = Error;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), Error> {
        match layout {
            Layout::Symbol => {
                let index = symbols.find(self);
                visitor.push_int(index as i64);
            }
            _ => return Err("expected symbol".to_string().into()),
        }

        Ok(())
    }
}

impl Encode for str {
    type Err = Error;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), Error> {
        match layout {
            Layout::Symbol => {
                let index = symbols.find(self);
                visitor.push_int(index as i64);
            }
            _ => return Err("expected symbol".to_string().into()),
        }

        Ok(())
    }
}

impl<T: Encode<Err = Error>> Encode for [T] {
    type Err = T::Err;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), T::Err> {
        match layout {
            Layout::List(element, size) => {
                if self.len() != *size {
                    return Err(format!(
                        "expected array of size {size}, got array of size {}",
                        self.len()
                    )
                    .into());
                }
                for item in self {
                    item.visit(element, symbols, visitor)?;
                }
            }
            _ => return Err("expected list".to_string().into()),
        }

        Ok(())
    }
}

impl<T: Encode<Err = Error>> Encode for Vec<T> {
    type Err = T::Err;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), T::Err> {
        match layout {
            Layout::List(element, size) => {
                if self.len() != *size {
                    return Err(format!(
                        "expected array of size {size}, got array of size {}",
                        self.len()
                    )
                    .into());
                }
                for item in self {
                    item.visit(element, symbols, visitor)?;
                }
            }
            _ => return Err("expected list".to_string().into()),
        }

        Ok(())
    }
}

impl<T: Encode<Err = Error>> Encode for HashMap<String, T> {
    type Err = T::Err;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), T::Err> {
        match layout {
            Layout::Struct(fields) => {
                for (name, field) in &fields.0 {
                    let Some(value) = self.get(name) else {
                        return Err(format!("missing field {name:?} in struct").into());
                    };
                    value.visit(field, symbols, visitor)?;
                }
            }
            _ => return Err("expected struct".to_string().into()),
        }

        Ok(())
    }
}

impl<T: Encode<Err = Error>> Encode for BTreeMap<String, T> {
    type Err = T::Err;
    fn visit(
        &self,
        layout: &Layout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), T::Err> {
        match layout {
            Layout::Struct(fields) => {
                for (name, field) in &fields.0 {
                    let Some(value) = self.get(name) else {
                        return Err(format!("missing field {name:?} in struct").into());
                    };
                    value.visit(field, symbols, visitor)?;
                }
            }
            _ => return Err("expected struct".to_string().into()),
        }

        Ok(())
    }
}

macro_rules! impl_encode_tuple {
    ($($n:tt: $typ:ident),*) => {
        impl< $( $typ, )* > Encode for ( $( $typ, )* ) where $($typ: Encode<Err = Error>),* {
            type Err = Error;
            fn visit(
                &self,
                layout: &Layout,
                symbols: &mut dyn Sym,
                visitor: &mut Visitor,
            ) -> Result<(), Error> {
                match layout {
                    Layout::Tuple(fields) => {
                        $(
                            self.$n.visit(
                                fields.get($n).ok_or_else(|| format!("missing field {} in tuple", $n))?,
                                symbols,
                                visitor,
                            )?;
                        )*
                    }
                    _ => return Err("expected a tuple".to_string().into()),
                }

                Ok(())
            }
        }
    };
}

impl_encode_tuple!(0: A);
impl_encode_tuple!(0: A, 1: B);
impl_encode_tuple!(0: A, 1: B, 2: C);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
impl_encode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K);

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
                        num.as_f64().ok_or_else(|| {
                            format!("{num} cannot be represented as 64 bit float")
                        })?,
                    )
                }
            }
            (Self::String(num), Layout::Scalar) if num.parse::<f64>().is_ok() => visitor.push(
                num.parse::<f64>()
                    .expect("can't fail because precondition was checked"),
            ),
            (Self::String(datetime), Layout::DateTime(format)) => {
                let timestamp = utils::Timestamp::from(
                    utils::parse_datetime(datetime, format)
                        .map_err(|err| err.to_string())?
                        .to_utc(),
                );
                visitor.push_int(timestamp.into());
            }
            (Self::String(e), Layout::Symbol) => {
                let index = symbols.find(e);
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
