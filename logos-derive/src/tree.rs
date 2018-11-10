use syn::Ident;
use quote::quote;
use proc_macro2::TokenStream;
use std::cmp::Ordering;
use regex::{Pattern, Parser};

use generator::{exhaustive, Generator, ExhaustiveGenerator, LooseGenerator};

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub pattern: Pattern,
    pub token: Option<&'a Ident>,
    pub consequents: Vec<Node<'a>>,
}

impl<'a> Node<'a> {
    pub fn new<P: Parser>(pattern: Pattern, path: &[u8], token: &'a Ident) -> Self {
        let mut node = Node {
            pattern,
            token: None,
            consequents: Vec::new(),
        };

        node.insert::<P>(path, token);

        node
    }

    pub fn insert<P: Parser>(&mut self, path: &[u8], token: &'a Ident) {
        if path.len() == 0 {
            // FIXME: Error on conflicting token stuff
            return self.token = Some(token);
        }

        let (pattern, read) = <P as Parser>::parse(path).expect("Invalid pattern!");

        if let Pattern::Repeat(_) = pattern {
            // FIXME: Error on conflicting token stuff
            self.token = Some(token);
        }

        match self.consequents.binary_search_by(|node| {
            (&node.pattern).partial_cmp(&pattern).unwrap_or_else(|| Ordering::Greater)
        }) {
            Ok(index) => {
                self.consequents[index].insert::<P>(&path[read..], token);
            },
            Err(index) => {
                self.consequents.insert(index, Node::new::<P>(pattern, &path[read..], token));
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

    pub fn insert<P: Parser>(&mut self, path: String, token: &'a Ident) {
        let path = &path.as_bytes()[1..path.len() - 1];

        if path.len() == 0 {
            panic!("#[token] value must not be empty.");
        }

        let (pattern, read) = <P as Parser>::parse(path).expect("Invalid pattern!");

        for byte in pattern {
            match &mut self.handlers[byte as usize] {
                &mut Handler::Tree(ref mut node) => node.insert::<P>(&path[read..], token),
                slot => *slot = Handler::Tree(Node::new::<P>(Pattern::Byte(byte), &path[read..], token))
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
