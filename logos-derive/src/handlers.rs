use syn::Ident;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Token<'a, T>(pub T, pub &'a Ident);

#[derive(Debug, Clone)]
pub enum Handler<'a> {
    Eof,
    Error,
    Whitespace,
    Tree {
        strings: Vec<Token<'a, String>>,
        regex: Option<Token<'a, Regex>>,
    },
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

    pub fn insert_string(&mut self, string: String, token: &'a Ident) {
        let byte = string.as_bytes()[0];
        let token = Token(string, token);

        match self.handlers[byte as usize] {
            Handler::Tree { ref mut strings, .. } => strings.push(token),
            ref mut slot => *slot = Handler::Tree {
                strings: vec![token],
                regex: None,
            }
        }
    }

    pub fn insert_regex(&mut self, mut regex: Regex, token: &'a Ident) {
        let first = regex.first();

        for byte in first {
            let token = Token(regex.clone(), token);

            match self.handlers[byte as usize] {
                Handler::Tree { ref mut regex, .. } => {
                    // FIXME: Panic if regex is already Some
                    *regex = Some(token);
                },
                ref mut slot => *slot = Handler::Tree {
                    strings: Vec::new(),
                    regex: Some(token),
                }
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
