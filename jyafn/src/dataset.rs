#![cfg(feature = "map-reduce")]
#![allow(unused_variables)]

use super::layout::{Decode, Decoder, Encode, Layout, Visitor, ZeroDecoder};
use super::{Error, Function, FunctionData};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct Dataset {
    layout: Layout,
    byte_size: usize,
    n_items: usize,
    raw: Vec<u8>,
}

impl Dataset {
    pub fn len(&self) -> usize {
        self.n_items
    }

    pub fn try_build<I, E, Err, C>(layout: Layout, mut conv_err: C, it: I) -> Result<Dataset, Err>
    where
        I: IntoIterator<Item = Result<E, Err>>,
        E: Encode,
        C: FnMut(Error) -> Err,
    {
        // let mut visitor = Visitor::new(layout.size());
        // let iter = it.into_iter();
        // let mut raw = Vec::with_capacity(usize::max(10, iter.size_hint().0));
        // let mut n_items = 0;

        // for item in iter {
        //     let item = item?;
        //     visitor.reset();
        //     item.visit(&layout, &mut visitor)
        //         .map_err(|err| Error::EncodeError(Box::new(err)))
        //         .map_err(&mut conv_err)?;
        //     raw.extend_from_slice(visitor.as_ref());
        //     n_items += 1;
        // }

        // Ok(Dataset {
        //     layout,
        //     byte_size: visitor.as_ref().len(),
        //     raw,
        //     n_items,
        // })
        unimplemented!()
    }

    pub fn build<I, E>(layout: Layout, it: I) -> Result<Dataset, Error>
    where
        I: IntoIterator<Item = E>,
        E: Encode,
    {
        Dataset::try_build(layout, |e| e, it.into_iter().map(Ok))
    }

    pub fn map(&self, func: &Function) -> Result<Dataset, Error> {
        if &self.layout != func.input_layout() {
            return Err(Error::WrongLayout {
                expected: self.layout.clone(),
                got: func.input_layout().clone(),
            });
        }

        let mut output_buffer = vec![0; func.output_size()];
        let mut output = Vec::with_capacity(self.n_items * func.output_size());

        for item in self.raw.chunks(self.byte_size) {
            let status = func.call_raw(item, &mut output_buffer);
            if status != 0 {
                return if let Some(error) = func.graph().errors.get((status - 1) as usize) {
                    Err(Error::StatusRaised(error.to_string()))
                } else {
                    Err(Error::StatusRaised(format!(
                        "unknown error of id {}",
                        status - 1
                    )))
                };
            }
            output.extend_from_slice(&output_buffer);
        }

        Ok(Dataset {
            layout: func.output_layout().clone(),
            byte_size: func.output_size(),
            n_items: self.n_items,
            raw: output,
        })
    }

    pub fn par_map(&self, func: &Function) -> Result<Dataset, Error> {
        if &self.layout != func.input_layout() {
            return Err(Error::WrongLayout {
                expected: self.layout.clone(),
                got: func.input_layout().clone(),
            });
        }

        let data: Arc<FunctionData> = func.into();
        let output_size = func.output_size();
        let mut output = vec![0; self.len() * output_size];
        let stride = (self.len() as f64 / num_cpus::get() as f64).ceil() as usize;
        let error = Mutex::new(None);

        rayon::scope(|s| {
            self.raw
                .chunks(stride * self.byte_size)
                .zip(output.chunks_mut(stride * output_size))
                .for_each(|(input_slice, output_slice)| {
                    s.spawn(|_| {
                        let func: Function = data.clone().into();
                        let input_outputs = input_slice
                            .chunks(self.byte_size)
                            .zip(output_slice.chunks_mut(output_size));

                        for (input, output) in input_outputs {
                            let status = func.call_raw(input, output);
                            if status != 0 {
                                *error.lock().expect("poisoned") = Some(
                                    if let Some(error) =
                                        func.graph().errors.get((status - 1) as usize)
                                    {
                                        Error::StatusRaised(error.to_string())
                                    } else {
                                        Error::StatusRaised(format!(
                                            "unknown error of id {}",
                                            status - 1
                                        ))
                                    },
                                );
                                return;
                            }
                        }
                    })
                })
        });

        if let Some(e) = error.into_inner().expect("poisoned") {
            return Err(e);
        }

        Ok(Dataset {
            layout: func.output_layout().clone(),
            byte_size: output_size,
            n_items: self.n_items,
            raw: output,
        })
    }

    pub fn decode_with_decoder<D>(&self, decoder: D) -> DecodeIter<D>
    where
        D: Decoder,
    {
        DecodeIter {
            dataset: self,
            decoder,
            item_pos: 0,
            visitor: RefCell::new(Visitor::new(self.byte_size / 8)),
        }
    }

    pub fn decode<D>(&self) -> DecodeIter<ZeroDecoder<D>>
    where
        D: Decode,
    {
        self.decode_with_decoder(ZeroDecoder::new())
    }
}

pub struct DecodeIter<'a, D> {
    dataset: &'a Dataset,
    decoder: D,
    item_pos: usize,
    visitor: RefCell<Visitor>,
}

impl<'a, D: Decoder> Iterator for DecodeIter<'a, D> {
    type Item = D::Target;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.dataset.n_items, Some(self.dataset.n_items))
    }

    fn next(&mut self) -> Option<D::Target> {
        // if self.item_pos >= self.dataset.raw.len() {
        //     return None;
        // }
        // let item = &self.dataset.raw[self.item_pos..self.item_pos + self.dataset.byte_size];
        // self.item_pos += self.dataset.byte_size;

        // // NOTE: this is a kinda unnecessary copy. Can be avoided with rethinking the
        // // Visitor API to work with slices.
        // let mut visitor = self.visitor.borrow_mut();
        // visitor.0.copy_from_slice(item);
        // visitor.reset();

        // Some(self.decoder.build(&self.dataset.layout, &mut visitor))
        unimplemented!()
    }
}

impl<'a, D: Decoder> ExactSizeIterator for DecodeIter<'a, D> {}
