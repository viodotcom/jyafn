use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::utils::murmur;

const HASH_SEED: u64 = 12345678;

/// Gives the "id" of a given jyafn symbol.
pub fn symbol_hash(s: &str) -> u64 {
    murmur::murmur_hash64a(s.as_bytes(), HASH_SEED)
}

/// An abstraction over a collection of imutable pieces of data that can be referenced by
/// id by the code inside a function.
pub trait Sym {
    /// Gets the id of a piece of text, creating a new id, if necessary. Once the text
    /// already exists inside the instance, the returned id must always be the same.
    fn find(&mut self, name: &str) -> u64;
    /// Gets a piece of text by id, returning `None` if it doesn't exist.
    fn get(&self, id: u64) -> Option<&str>;
}

#[derive(Serialize, Deserialize)]
struct SymbolsSerde(Vec<String>);

impl From<SymbolsSerde> for Symbols {
    fn from(serde: SymbolsSerde) -> Symbols {
        let map = serde
            .0
            .iter()
            .cloned()
            .map(|name| {
                let id = symbol_hash(&name);
                (id, name)
            })
            .collect();
        Symbols(map)
    }
}

impl From<Symbols> for SymbolsSerde {
    fn from(value: Symbols) -> Self {
        SymbolsSerde(value.0.into_values().collect())
    }
}

/// A collection of immutable pieces of data that can be referenced by id by the code
/// inside a function.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, GetSize)]
#[serde(from = "SymbolsSerde")]
#[serde(into = "SymbolsSerde")]
pub struct Symbols(BTreeMap<u64, String>);

impl Sym for Symbols {
    fn find(&mut self, name: &str) -> u64 {
        let h = symbol_hash(name);
        self.0.insert(h, name.to_owned());
        h
    }

    fn get(&self, id: u64) -> Option<&str> {
        self.0.get(&id).map(String::as_str)
    }
}

impl Symbols {
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Adds a new symbol into this collection symbols.
    pub fn push(&mut self, symbol: String) -> u64 {
        let h = symbol_hash(&symbol);
        self.0.insert(h, symbol);
        h
    }

    /// Creates a view over these symbols, on top of which extra symbols can be added.
    pub fn view(&self) -> SymbolsView {
        SymbolsView::new(self)
    }

    pub fn as_vec(&self) -> Vec<String> {
        self.0.values().cloned().collect()
    }
}

/// A view on top of an already existing [`Symbols`]. This allows immutable access to the
/// existing symbols while temporarily allocating the new ones that might appear.
pub struct SymbolsView<'a> {
    top: &'a Symbols,
    extra: Option<Symbols>,
}

impl SymbolsView<'_> {
    /// Creates a new view from an immutable reference to a collection of symbols.
    pub(crate) fn new(s: &Symbols) -> SymbolsView {
        SymbolsView {
            top: s,
            extra: None,
        }
    }

    /// Returns the extra allocated symbols for this view.
    pub fn into_extra(self) -> Symbols {
        self.extra.unwrap_or_default()
    }
}

impl Sym for SymbolsView<'_> {
    fn find(&mut self, name: &str) -> u64 {
        let h = symbol_hash(name);

        if self.top.0.contains_key(&h) {
            h
        } else if let Some(extra) = self.extra.as_mut() {
            extra.0.insert(h, name.to_string());
            h
        } else {
            let mut extra = Symbols::default();
            extra.0.insert(h, name.to_string());
            self.extra = Some(extra);

            h
        }
    }

    fn get(&self, id: u64) -> Option<&str> {
        if let Some(name) = self.top.get(id) {
            Some(name)
        } else if let Some(extra) = self.extra.as_ref() {
            extra.get(id)
        } else {
            None
        }
    }
}
