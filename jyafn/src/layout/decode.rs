use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use hashbrown::HashMap;

use crate::utils;

use super::symbols::Sym;
use super::{Layout, Visitor};

/// Decodes unstructured binary data into a target data structure.
pub trait Decoder {
    /// The tartget type to be built.
    type Target;
    /// Decodes unstructured data stored inside `visitor`, given a broader context of a
    /// `layout` and `symbols`, to produce a target data type.
    ///
    /// The input parameters are  already guaranteed to be correctly formed. Therefore,
    /// no decode errors are expected from this function. If necessary, this code should
    /// panic, indicating a bug in the caller code.
    fn build(&mut self, layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self::Target;
}

/// A type that can be decoded from a `layout`, `symbols` and a visitor.
pub trait Decode {
    /// Creates a value of `Self` corresponding to the supplied information.
    ///
    /// The input parameters are  already guaranteed to be correctly formed. Therefore,
    /// no decode errors are expected from this function. If necessary, this code should
    /// panic, indicating a bug in the caller code.
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self;
}

/// A decoder for types that implement [`Decode`].
#[derive(Debug, Clone, Copy)]
pub struct ZeroDecoder<D>(std::marker::PhantomData<D>);

impl<D> Default for ZeroDecoder<D> {
    fn default() -> Self {
        ZeroDecoder(std::marker::PhantomData)
    }
}

impl<D> ZeroDecoder<D> {
    pub fn new() -> Self {
        ZeroDecoder::default()
    }
}

impl<D: Decode> Decoder for ZeroDecoder<D> {
    type Target = D;
    fn build(&mut self, layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self::Target {
        D::build(layout, symbols, visitor)
    }
}

impl Decode for () {
    fn build(layout: &Layout, _: &dyn Sym, _: &mut Visitor) -> Self {
        match layout {
            Layout::Unit => {}
            _ => panic!("Bad layout for (): {layout:?}"),
        }
    }
}

macro_rules! impl_decode_container {
    ($container:ty) => {
        impl<T: Decode> Decode for $container {
            fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
                Self::from(T::build(layout, symbols, visitor))
            }
        }
    };
}

impl_decode_container!(Box<T>);
impl_decode_container!(Rc<T>);
impl_decode_container!(Arc<T>);

macro_rules! impl_decode_scalar {
    ($scalar:ty) => {
        impl Decode for $scalar {
            fn build(layout: &Layout, _: &dyn Sym, visitor: &mut Visitor) -> Self {
                match layout {
                    Layout::Scalar => visitor.pop() as $scalar,
                    _ => panic!("Bad layout for {}: {layout:?}", stringify!($scalar)),
                }
            }
        }
    };
}

impl_decode_scalar!(i8);
impl_decode_scalar!(u8);
impl_decode_scalar!(i16);
impl_decode_scalar!(u16);
impl_decode_scalar!(i32);
impl_decode_scalar!(u32);
impl_decode_scalar!(i64);
impl_decode_scalar!(u64);
impl_decode_scalar!(isize);
impl_decode_scalar!(usize);
impl_decode_scalar!(f64);
impl_decode_scalar!(f32);

impl Decode for bool {
    fn build(layout: &Layout, _: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Bool => match visitor.pop_int() {
                0 => false,
                1 => true,
                i => panic!("Bad integer value for bool: {i}"),
            },
            _ => panic!("Bad layout for bool: {layout:?}"),
        }
    }
}

impl Decode for String {
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Symbol => {
                let index = visitor.pop_uint();
                let Some(string) = symbols.get(index) else {
                    panic!("Symbol of index {index} not found")
                };
                string.to_owned()
            }
            _ => panic!("Bad layout for String: {layout:?}"),
        }
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::List(layout, size) => (0..*size)
                .map(|_| T::build(layout, symbols, visitor))
                .collect(),
            _ => panic!("Bad layout for Vec<_>: {layout:?}"),
        }
    }
}

macro_rules! impl_decode_tuple {
    ($($n:tt: $typ:ident),*) => {
        impl< $( $typ, )* > Decode for ( $( $typ, )* ) where $($typ: Decode),* {
            fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
                match layout {
                    Layout::Tuple(fields) => {
                        (
                            $({
                                let Some(field_layout) = fields.get($n) else {
                                    panic!("Missing field {} in tuple layout", $n)
                                };
                                $typ::build(field_layout, symbols, visitor)
                            },)*
                        )
                    },
                    _ => panic!("Bad layout for tuple: {layout:?}"),
                }
            }
        }
    }
}

impl_decode_tuple!(0: A);
impl_decode_tuple!(0: A, 1: B);
impl_decode_tuple!(0: A, 1: B, 2: C);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
impl_decode_tuple!(0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K);

impl<T: Decode> Decode for HashMap<String, T> {
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Struct(fields) => {
                let mut decoded = HashMap::with_capacity(fields.0.len());

                for (name, field) in &fields.0 {
                    decoded.insert(name.to_owned(), T::build(field, symbols, visitor));
                }

                decoded
            }
            _ => panic!("Bad layout for HashMap<String, _>: {layout:?}"),
        }
    }
}

impl<T: Decode> Decode for BTreeMap<String, T> {
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Struct(fields) => {
                let mut decoded = BTreeMap::new();

                for (name, field) in &fields.0 {
                    decoded.insert(name.to_owned(), T::build(field, symbols, visitor));
                }

                decoded
            }
            _ => panic!("Bad layout for BTreeMap<String, _>: {layout:?}"),
        }
    }
}

impl Decode for serde_json::Value {
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Unit => Self::Null,
            Layout::Scalar => visitor.pop().into(),
            Layout::Bool => (visitor.pop_int() != 0).into(),
            Layout::DateTime(format) => {
                chrono::DateTime::<chrono::Utc>::from(utils::Timestamp::from(visitor.pop_int()))
                    .format(format)
                    .to_string()
                    .into()
            }
            Layout::Symbol => Self::String(symbols.get(visitor.pop_uint()).unwrap().to_string()),
            Layout::Struct(fields) => fields
                .0
                .iter()
                .map(|(name, field)| (name.clone(), Self::build(field, symbols, visitor)))
                .collect::<serde_json::Map<_, _>>()
                .into(),
            Layout::Tuple(fields) => fields
                .iter()
                .map(|field| Self::build(field, symbols, visitor))
                .collect::<Vec<_>>()
                .into(),
            Layout::List(element, size) => (0..*size)
                .map(|_| Self::build(element, symbols, visitor))
                .collect::<Vec<_>>()
                .into(),
        }
    }
}
