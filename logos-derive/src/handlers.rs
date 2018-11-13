use syn::Ident;
use regex::Regex;
use tree::Node;

#[derive(Debug, Clone)]
pub enum Handler<'a> {
    Eof,
    Error,
    Whitespace,
    Tree(Node<'a>),
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

    pub fn insert(&mut self, mut regex: Regex, token: &'a Ident) {
        let first = regex.next().expect("Cannot assign tokens to empty patterns");

        for byte in first {
            let regex = regex.clone();

            match self.handlers[byte as usize] {
                Handler::Tree(ref mut root) => root.insert(Node::new(regex, token)),
                ref mut slot => *slot = Handler::Tree(Node::new(regex, token)),
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
