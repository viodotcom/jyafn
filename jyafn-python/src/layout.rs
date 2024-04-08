use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rust::layout::{Decoder, Encode, Layout as RustLayout, Sym, Visitor};

pub struct Obj<'py>(pub Bound<'py, PyAny>);

impl<'py> Encode for Obj<'py> {
    type Err = PyErr;
    fn visit(
        &self,
        layout: &RustLayout,
        symbols: &mut dyn Sym,
        visitor: &mut Visitor,
    ) -> Result<(), PyErr> {
        match layout {
            RustLayout::Scalar => {
                if let Ok(float) = self.0.extract::<f64>() {
                    visitor.push(float);
                } else {
                    return Err(exceptions::PyTypeError::new_err(format!(
                        "expected {layout}, got {}",
                        self.0
                    )));
                }
            }
            RustLayout::Bool => {
                if let Ok(float) = self.0.extract::<bool>() {
                    visitor.push_int(float as i64);
                } else {
                    return Err(exceptions::PyTypeError::new_err(format!(
                        "expected {layout}, got {}",
                        self.0
                    )));
                }
            }
            RustLayout::Struct(fields) => {
                for (name, field) in &fields.0 {
                    let Ok(item) = self.0.get_item(name) else {
                        return Err(exceptions::PyTypeError::new_err(format!(
                            "missing field {name:?} in {}",
                            self.0
                        )));
                    };
                    Obj(item).visit(field, symbols, visitor)?;
                }
            }
            RustLayout::Symbol => {
                let e = self.0.extract::<String>()?;
                let Some(index) = symbols.find(&*e) else {
                    return Err(exceptions::PyTypeError::new_err(format!(
                        "symbol {e:?} not found",
                    )));
                };
                visitor.push_int(index as i64);
            }
            RustLayout::List(element, size) => {
                let mut n_items = 0;
                for item in self.0.iter()? {
                    let item = item?;

                    Obj(item).visit(element, symbols, visitor)?;

                    n_items += 1;
                }

                if n_items != *size {
                    return Err(exceptions::PyTypeError::new_err(format!(
                        "expected array of size {size}, got array of size {n_items}",
                    )));
                }
            }
            _ => {
                return Err(exceptions::PyTypeError::new_err(format!(
                    "incompatible layout {layout} for {}",
                    self.0
                )))
            }
        }

        Ok(())
    }
}

pub struct PyDecoder<'py>(pub Python<'py>);

impl<'py> Decoder for PyDecoder<'py> {
    type Target = PyObject;
    fn build(
        &mut self,
        layout: &RustLayout,
        symbols: &dyn Sym,
        visitor: &mut Visitor,
    ) -> Self::Target {
        match layout {
            RustLayout::Unit => ().to_object(self.0),
            RustLayout::Scalar => visitor.pop().to_object(self.0),
            RustLayout::Bool => (visitor.pop_int() != 0).to_object(self.0),
            RustLayout::Struct(fields) => {
                let dict = pyo3::types::PyDict::new_bound(self.0);

                for (name, field) in &fields.0 {
                    dict.set_item(name, self.build(field, symbols, visitor))
                        .unwrap();
                }

                dict.to_object(self.0)
            }
            RustLayout::Symbol => symbols
                .get(visitor.pop_int() as usize)
                .unwrap()
                .to_object(self.0),
            RustLayout::List(element, size) => pyo3::types::PyList::new_bound(
                self.0,
                (0..*size).map(|_| self.build(element, symbols, visitor)),
            )
            .to_object(self.0),
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Layout(pub(crate) rust::layout::Layout);

#[pymethods]
impl Layout {
    fn __repr__(&self) -> String {
        format!(
            "Layout({})",
            serde_json::to_string(&self.0).expect("can always serialize")
        )
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn is_unit(&self) -> bool {
        self.0 == rust::layout::Layout::Unit
    }

    fn is_scalar(&self) -> bool {
        self.0 == rust::layout::Layout::Scalar
    }

    fn is_symbol(&self) -> bool {
        matches!(&self.0, rust::layout::Layout::Symbol)
    }

    fn struct_keys(&self, py: Python) -> PyResult<PyObject> {
        let rust::layout::Layout::Struct(s) = &self.0 else {
            return Ok(pyo3::types::PyNone::get_bound(py).to_object(py));
        };

        let list = pyo3::types::PyList::new_bound(py, s.0.iter().map(|(name, _)| name.clone()));

        Ok(list.to_object(py))
    }

    #[staticmethod]
    fn unit() -> Layout {
        Layout(rust::layout::Layout::Unit)
    }

    #[staticmethod]
    fn scalar() -> Layout {
        Layout(rust::layout::Layout::Scalar)
    }

    #[staticmethod]
    fn symbol() -> Layout {
        Layout(rust::layout::Layout::Symbol)
    }

    #[staticmethod]
    fn list_of(element: &Layout, size: usize) -> Layout {
        Layout(rust::layout::Layout::List(
            Box::new(element.0.clone()),
            size,
        ))
    }

    #[staticmethod]
    fn struct_of(fields: &Bound<'_, PyDict>) -> PyResult<Layout> {
        let fields = fields
            .iter()
            .map(|(key, value)| Ok((key.extract::<String>()?, value.extract::<Layout>()?.0)))
            .collect::<PyResult<Vec<(String, rust::layout::Layout)>>>()?;

        Ok(Layout(rust::layout::Layout::Struct(rust::layout::Struct(
            fields,
        ))))
    }
}
