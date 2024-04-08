use super::symbols::Sym;
use super::{Layout, Visitor};

pub trait Decoder {
    type Target;
    fn build(&mut self, layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self::Target;
}

pub trait Decode {
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self;
}

#[derive(Debug, Clone, Copy)]
pub struct ZeroDecoder<D>(std::marker::PhantomData<D>);

impl<D> ZeroDecoder<D> {
    pub fn new() -> Self {
        ZeroDecoder(std::marker::PhantomData)
    }
}

impl<D: Decode> Decoder for ZeroDecoder<D> {
    type Target = D;
    fn build(&mut self, layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self::Target {
        D::build(layout, symbols, visitor)
    }
}

impl Decode for f64 {
    fn build(layout: &Layout, _: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Scalar => visitor.pop(),
            _ => panic!("Bad layout for f64: {layout:?}"),
        }
    }
}

impl Decode for serde_json::Value {
    fn build(layout: &Layout, symbols: &dyn Sym, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Unit => Self::Null,
            Layout::Scalar => visitor.pop().into(),
            Layout::Bool => (visitor.pop_int() != 0).into(),
            Layout::Struct(fields) => fields
                .0
                .iter()
                .map(|(name, field)| (name.clone(), Self::build(field, symbols, visitor)))
                .collect::<serde_json::Map<_, _>>()
                .into(),
            Layout::Symbol => {
                Self::String(symbols.get(visitor.pop_int() as usize).unwrap().to_string())
            }
            Layout::List(element, size) => (0..*size)
                .map(|_| Self::build(element, symbols, visitor))
                .collect::<Vec<_>>()
                .into(),
        }
    }
}
