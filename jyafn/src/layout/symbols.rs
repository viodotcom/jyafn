use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::convert::AsRef;

/// An abstraction over a collection of imutable pieces of data that can be referenced by
/// id by the code inside a function.
pub trait Sym {
    /// Gets the id of a piece of text, creating a new id, if necessary. Once the text
    /// already exists inside the instance, the returned id must always be the same.
    fn find(&mut self, name: &str) -> usize;
    /// Gets a piece of text by id, returning `None` if it doesn't exist.
    fn get(&self, id: usize) -> Option<&str>;
}

/// A collection of immutable pieces of data that can be referenced by id by the code
/// inside a function.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct Symbols(Vec<String>);

impl Sym for Symbols {
    fn find(&mut self, name: &str) -> usize {
        if let Some(symbol_id) = self.0.iter().position(|e| e == name) {
            symbol_id
        } else {
            let symbol_id = self.0.len();
            self.0.push(name.to_string());
            symbol_id
        }
    }

    fn get(&self, id: usize) -> Option<&str> {
        self.0.get(id).map(String::as_str)
    }
}

impl AsRef<[String]> for Symbols {
    fn as_ref(&self) -> &[String] {
        &self.0
    }
}

impl Symbols {
    /// Adds a new symbol into this collection symbols.
    pub fn push(&mut self, symbol: String) -> usize {
        if let Some(symbol_id) = self.0.iter().position(|e| e == &symbol) {
            symbol_id
        } else {
            let symbol_id = self.0.len();
            self.0.push(symbol);
            symbol_id
        }
    }
}

/// A view on top of an already existing [`Symbols`]. This allows immutable access to the
/// existing symbols while temporarily allocating the new ones that might appear.
pub(crate) struct SymbolsView<'a> {
    top: &'a Symbols,
    new: Option<Symbols>,
}

impl SymbolsView<'_> {
    /// Creates a new view from an immutable reference to a collection of symbols.
    pub(crate) fn new(s: &Symbols) -> SymbolsView {
        SymbolsView { top: s, new: None }
    }
}

impl Sym for SymbolsView<'_> {
    fn find(&mut self, name: &str) -> usize {
        if let Some(id) = self.top.0.iter().position(|e| e == name) {
            id
        } else if let Some(new) = self.new.as_mut() {
            new.push(name.to_string()) + self.top.0.len()
        } else {
            let mut new = Symbols::default();
            let id = new.push(name.to_string());
            self.new = Some(new);

            id + self.top.0.len()
        }
    }

    fn get(&self, id: usize) -> Option<&str> {
        if let Some(name) = self.top.get(id) {
            Some(name)
        } else if let Some(new) = self.new.as_ref() {
            new.get(id - self.top.0.len())
        } else {
            None
        }
    }
}
