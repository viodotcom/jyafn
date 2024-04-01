use byte_slice_cast::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Struct(pub Vec<(String, Layout)>);

impl Struct {
    pub fn size(&self) -> usize {
        self.0.iter().map(|(_, layout)| layout.size()).sum()
    }

    pub fn insert(&mut self, name: String, field: Layout) {
        self.0.push((name, field))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    Unit,
    Scalar,
    Struct(Box<Struct>),
    Enum(Vec<String>),
    List(Box<Layout>, usize),
}

impl From<Struct> for Layout {
    fn from(fields: Struct) -> Layout {
        Layout::Struct(Box::new(fields))
    }
}

impl Layout {
    pub fn size(&self) -> usize {
        match self {
            Layout::Unit => 0,
            Layout::Scalar => 1,
            Layout::Struct(fields) => fields.size(),
            Layout::Enum(_) => 1,
            Layout::List(element, size) => size * element.size(),
        }
    }
}

pub enum Value {
    Unit,
    Scalar(f64),
    Struct(HashMap<String, Value>),
    Enum(String),
    List(Vec<Value>),
}

#[derive(Debug, Clone)]
pub struct Visitor(pub(crate) Box<[u8]>, usize);

impl From<Box<[u8]>> for Visitor {
    fn from(b: Box<[u8]>) -> Visitor {
        Visitor(b, 0)
    }
}

impl Visitor {
    pub(crate) fn new(size: usize) -> Visitor {
        Visitor(vec![0; size * 8].into_boxed_slice(), 0)
    }

    pub(crate) fn new_like(other: &Visitor) -> Visitor {
        Visitor(vec![0; other.0.len()].into_boxed_slice(), 0)
    }

    pub fn as_ref(&self) -> &[u8] {
        &self.0
    }

    pub fn into_inner(self) -> Box<[u8]> {
        self.0
    }

    pub(crate) fn reset(&mut self) {
        self.1 = 0
    }

    pub fn push(&mut self, val: f64) {
        self.0.as_mut_slice_of::<f64>().unwrap()[self.1] = val;
        self.1 += 1;
    }

    pub fn pop(&mut self) -> f64 {
        let top = self.0.as_mut_slice_of::<f64>().unwrap()[self.1];
        self.1 += 1;
        top
    }
}

pub trait Encode {
    fn visit(&self, layout: &Layout, visitor: &mut Visitor) -> Result<(), ()>;
}

pub trait Decoder {
    type Target;
    fn build(&mut self, layout: &Layout, visitor: &mut Visitor) -> Self::Target;
}

pub trait Decode {
    fn build(layout: &Layout, visitor: &mut Visitor) -> Self;
}

impl Encode for f64 {
    fn visit(&self, layout: &Layout, visitor: &mut Visitor) -> Result<(), ()> {
        match layout {
            Layout::Scalar => visitor.push(*self),
            _ => return Err(()),
        }

        Ok(())
    }
}

impl Encode for Value {
    fn visit(&self, layout: &Layout, visitor: &mut Visitor) -> Result<(), ()> {
        match (self, layout) {
            (Self::Unit, Layout::Unit) => {}
            (Self::Scalar(s), Layout::Scalar) => visitor.push(*s),
            (Self::Enum(e), Layout::Enum(options)) => {
                let Some(index) = options.iter().position(|o| o == e) else {
                    return Err(());
                };
                visitor.push(index as f64);
            }
            (Self::List(array), Layout::List(element, size)) => {
                if array.len() != *size {
                    return Err(());
                }
                for item in array {
                    item.visit(element, visitor)?;
                }
            }
            (Self::Struct(map), Layout::Struct(fields)) => {
                for (name, field) in &fields.0 {
                    let Some(value) = map.get(name) else {
                        return Err(());
                    };
                    value.visit(field, visitor)?;
                }
            }
            _ => return Err(()),
        }

        Ok(())
    }
}
impl Encode for serde_json::Value {
    fn visit(&self, layout: &Layout, visitor: &mut Visitor) -> Result<(), ()> {
        match (self, layout) {
            (Self::Null, Layout::Unit) => {}
            (Self::Bool(b), Layout::Scalar) => {
                visitor.push(if *b { 1.0 } else { 0.0 });
            }
            (Self::Number(num), Layout::Scalar) => {
                if let Some(int) = num.as_i64() {
                    visitor.push(int as f64)
                } else if let Some(uint) = num.as_u64() {
                    visitor.push(uint as f64)
                } else {
                    visitor.push(num.as_f64().expect("number must be a float"))
                }
            }
            (Self::String(string), Layout::Enum(options)) => {
                let Some(index) = options.iter().position(|o| o == string) else {
                    return Err(());
                };
                visitor.push(index as f64);
            }
            (Self::Array(array), Layout::List(element, size)) => {
                if array.len() != *size {
                    return Err(());
                }
                for item in array {
                    item.visit(element, visitor)?;
                }
            }
            (Self::Object(map), Layout::Struct(fields)) => {
                for (name, field) in &fields.0 {
                    let Some(value) = map.get(name) else {
                        return Err(());
                    };
                    value.visit(field, visitor)?;
                }
            }
            _ => return Err(()),
        }

        Ok(())
    }
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
    fn build(&mut self, layout: &Layout, visitor: &mut Visitor) -> Self::Target {
        D::build(layout, visitor)
    }
}

impl Decode for f64 {
    fn build(layout: &Layout, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Scalar => visitor.pop(),
            _ => panic!("Bad layout for f64: {layout:?}"),
        }
    }
}

impl Decode for Value {
    fn build(layout: &Layout, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Unit => Self::Unit,
            Layout::Scalar => Self::Scalar(visitor.pop()),
            Layout::Struct(fields) => Self::Struct(
                fields
                    .0
                    .iter()
                    .map(|(name, field)| (name.clone(), Self::build(field, visitor)))
                    .collect::<HashMap<_, _>>(),
            ),
            Layout::Enum(_) => todo!(),
            Layout::List(element, size) => Value::List(
                (0..*size)
                    .map(|_| Self::build(element, visitor))
                    .collect::<Vec<_>>(),
            ),
        }
    }
}

impl Decode for serde_json::Value {
    fn build(layout: &Layout, visitor: &mut Visitor) -> Self {
        match layout {
            Layout::Unit => Self::Null,
            Layout::Scalar => visitor.pop().into(),
            Layout::Struct(fields) => fields
                .0
                .iter()
                .map(|(name, field)| (name.clone(), Self::build(field, visitor)))
                .collect::<serde_json::Map<_, _>>()
                .into(),
            Layout::Enum(_) => todo!(),
            Layout::List(element, size) => (0..*size)
                .map(|_| Self::build(element, visitor))
                .collect::<Vec<_>>()
                .into(),
        }
    }
}
