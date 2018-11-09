use syn::Ident;
use quote::quote;
use proc_macro2::TokenStream;

use generator::{exhaustive, Generator, ExhaustiveGenerator, LooseGenerator};

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub byte: u8,
    pub token: Option<&'a Ident>,
    pub consequents: Vec<Node<'a>>,
}

impl<'a> Node<'a> {
    pub fn new(path: &[u8], token: &'a Ident) -> Self {
        let byte = path[0];

        let mut node = Node {
            byte,
            token: None,
            consequents: Vec::new(),
        };

        node.insert(&path[1..], token);

        node
    }

    pub fn insert(&mut self, path: &[u8], token: &'a Ident) {
        if path.len() == 0 {
            // FIXME: Error on conflicting token stuff
            return self.token = Some(token);
        }

        let byte = path[0];

        match self.consequents.binary_search_by(|node| node.byte.cmp(&byte)) {
            Ok(index) => {
                self.consequents[index].insert(&path[1..], token);
            },
            Err(index) => {
                self.consequents.insert(index, Node::new(path, token));
            },
        }

    }

    pub fn print(&self, name: &Ident) -> TokenStream {
        let body = if exhaustive(self) {
            ExhaustiveGenerator::print(self, name)
        } else {
            LooseGenerator::print(self, name)
        };

        quote!{ Some( #body ) }
    }
}

#[derive(Debug, Clone)]
pub enum Handler<'a> {
    Eof,
    Error,
    Whitespace,
    Tree(Node<'a>)
}

#[derive(Debug)]
pub struct Handlers<'a> {
    handlers: Vec<Handler<'a>>,
}

impl<'a> Handlers<'a> {
    pub fn new() -> Self {
        let mut handlers = vec![Handler::Error; 256];

        handlers[0] = Handler::Eof;
        handlers[1..33].iter_mut().for_each(|slot| *slot = Handler::Whitespace);

        Handlers {
            handlers
        }
    }

    pub fn insert(&mut self, path: String, token: &'a Ident) {
        let path = &path.as_bytes()[1..path.len() - 1];

        if path.len() == 0 {
            panic!("#[token] value must not be empty.");
        }

        let byte = path[0];

        match &mut self.handlers[byte as usize] {
            &mut Handler::Tree(ref mut node) => node.insert(&path[1..], token),
            slot => *slot = Handler::Tree(Node::new(path, token))
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
