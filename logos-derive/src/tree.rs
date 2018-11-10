use syn::Ident;
use quote::quote;
use proc_macro2::TokenStream;
use std::cmp::Ordering;
use regex::Pattern;

use generator::{exhaustive, Generator, ExhaustiveGenerator, LooseGenerator};

#[derive(Debug, Clone)]
pub struct Node<'a> {
    pub pattern: Pattern,
    pub token: Option<&'a Ident>,
    pub consequents: Vec<Node<'a>>,
}

impl<'a> Node<'a> {
    pub fn new<P>(pattern: Pattern, path: &mut P, token: &'a Ident) -> Self
    where
        P: Iterator<Item = Pattern>,
    {
        let mut node = Node {
            pattern,
            token: None,
            consequents: Vec::new(),
        };

        node.insert(path, token);

        node
    }

    pub fn insert<P>(&mut self, path: &mut P, token: &'a Ident)
    where
        P: Iterator<Item = Pattern>,
    {
        let pattern = match path.next() {
            Some(pattern) => pattern,
            None => {
                // FIXME: Error on conflicting token stuff
                return self.token = Some(token);
            }
        };

        if let Pattern::Repeat(_) = pattern {
            // FIXME: Error on conflicting token stuff
            self.token = Some(token);
        }

        match self.consequents.binary_search_by(|node| {
            (&node.pattern).partial_cmp(&pattern).unwrap_or_else(|| Ordering::Greater)
        }) {
            Ok(index) => {
                self.consequents[index].insert(path, token);
            },
            Err(index) => {
                self.consequents.insert(index, Node::new(pattern, path, token));
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

    pub fn insert<P>(&mut self, path: &mut P, token: &'a Ident)
    where
        P: Iterator<Item = Pattern> + Copy,
    {
        let pattern = path.next().expect("#[token] value must not be empty.");

        for byte in pattern {
            let mut path = *path;

            match &mut self.handlers[byte as usize] {
                &mut Handler::Tree(ref mut node) => node.insert(&mut path, token),
                slot => {
                    *slot = Handler::Tree(Node::new(Pattern::Byte(byte), &mut path, token));
                }
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
