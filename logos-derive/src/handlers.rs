use syn::Ident;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Branch<'a, T>(pub T, pub &'a Ident);

#[derive(Debug, Clone)]
pub enum Handler<'a> {
    Eof,
    Error,
    Whitespace,
    Tree(Tree<'a>),
}

#[derive(Debug, Clone)]
pub struct Tree<'a> {
    pub strings: Vec<Branch<'a, String>>,
    pub regex: Option<Branch<'a, Regex>>,
}

impl<'a> From<Tree<'a>> for Handler<'a> {
    fn from(tree: Tree<'a>) -> Handler<'a> {
        Handler::Tree(tree)
    }
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
        let branch = Branch(string, token);

        match self.handlers[byte as usize] {
            Handler::Tree(ref mut tree) => {
                tree.strings.push(branch);
            }
            ref mut slot => {
                *slot = Tree {
                    strings: vec![branch],
                    regex: None,
                }.into()
            }
        }
    }

    pub fn insert_regex(&mut self, mut regex: Regex, token: &'a Ident) {
        let first = regex.first();

        for byte in first {
            let branch = Branch(regex.clone(), token);

            match self.handlers[byte as usize] {
                Handler::Tree(ref mut tree) => {
                    // FIXME: Panic if regex is already Some
                    tree.regex = Some(branch);
                },
                ref mut slot => *slot = Tree {
                    strings: Vec::new(),
                    regex: Some(branch),
                }.into()
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
