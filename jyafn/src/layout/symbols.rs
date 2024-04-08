use serde_derive::{Deserialize, Serialize};

pub trait Sym {
    fn find(&mut self, name: &str) -> Option<usize>;
    fn get(&self, id: usize) -> Option<&str>;
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Symbols(Vec<String>);

impl Sym for Symbols {
    fn find(&mut self, name: &str) -> Option<usize> {
        if let Some(symbol_id) = self.0.iter().position(|e| e == name) {
            Some(symbol_id)
        } else {
            let symbol_id = self.0.len();
            self.0.push(name.to_string());
            Some(symbol_id)
        }
    }

    fn get(&self, id: usize) -> Option<&str> {
        self.0.get(id).map(String::as_str)
    }
}

impl Symbols {
    pub fn push(&mut self, symbol: String) -> usize {
        if let Some(symbol_id) = self.0.iter().position(|e| e == &symbol) {
            symbol_id
        } else {
            let symbol_id = self.0.len();
            self.0.push(symbol);
            symbol_id
        }
    }

    pub fn as_ref(&self) -> &[String] {
        &self.0
    }
}

pub(crate) struct SymbolsView<'a> {
    top: &'a Symbols,
    new: Option<Symbols>,
}

impl SymbolsView<'_> {
    pub(crate) fn new(s: &Symbols) -> SymbolsView {
        SymbolsView { top: s, new: None }
    }
}

impl Sym for SymbolsView<'_> {
    fn find(&mut self, name: &str) -> Option<usize> {
        if let Some(id) = self.top.0.iter().position(|e| e == name) {
            Some(id)
        } else if let Some(new) = self.new.as_mut() {
            Some(new.push(name.to_string()) + self.top.0.len())
        } else {
            let mut new = Symbols::default();
            let id = new.push(name.to_string());
            self.new = Some(new);
            Some(id + self.top.0.len())
        }
    }

    fn get(&self, id: usize) -> Option<&str> {
        if let Some(name) = self.top.get(id) {
            Some(name)
        } else if let Some(new) = self.new.as_ref() {
            new.get(id)
        } else {
            None
        }
    }
}
