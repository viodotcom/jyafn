use pyo3::prelude::*;
use rust::layout::{Decoder, Encode, Layout, Visitor};

pub struct Obj<'py>(pub Bound<'py, PyAny>);

impl<'py> Encode for Obj<'py> {
    fn visit(&self, layout: &Layout, visitor: &mut Visitor) -> Result<(), ()> {
        match layout {
            Layout::Scalar => {
                if let Ok(float) = self.0.extract::<f64>() {
                    visitor.push(float);
                } else {
                    return Err(());
                }
            }
            Layout::Struct(fields) => {
                for (name, field) in &fields.0 {
                    let Ok(item) = self.0.get_item(name) else {
                        return Err(());
                    };
                    Obj(item).visit(field, visitor)?;
                }
            }
            _ => return Err(()),
        }

        Ok(())
    }
}

pub struct PyDecoder<'py>(pub Python<'py>);

impl<'py> Decoder for PyDecoder<'py> {
    type Target = PyObject;
    fn build(&mut self, layout: &Layout, visitor: &mut Visitor) -> Self::Target {
        match layout {
            Layout::Unit => ().to_object(self.0),
            Layout::Scalar => visitor.pop().to_object(self.0),
            Layout::Struct(fields) => {
                let dict = pyo3::types::PyDict::new_bound(self.0);

                for (name, field) in &fields.0 {
                    dict.set_item(name, self.build(field, visitor)).unwrap();
                }

                dict.to_object(self.0)
            }
            Layout::Enum(_) => todo!(),
            Layout::List(element, size) => pyo3::types::PyList::new_bound(
                self.0,
                (0..*size).map(|_| self.build(element, visitor)),
            )
            .to_object(self.0),
        }
    }
}
