use syn::Ident;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Branch<'a>(pub Regex, pub &'a Ident);

#[derive(Debug, Clone)]
pub enum Handler<'a> {
    Eof,
    Error,
    Whitespace,
    Tree(Tree<'a>),
}

#[derive(Debug, Clone)]
pub struct Tree<'a> {
    pub branches: Vec<Branch<'a>>,
    pub regex: Option<Branch<'a>>,
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

    pub fn insert(&mut self, regex: Regex, token: &'a Ident) {
        let first = regex.patterns()[0].clone();

        for byte in first {
            let branch = Branch(regex.clone(), token);

            match self.handlers[byte as usize] {
                Handler::Tree(ref mut tree) => {
                    tree.branches.push(branch);
                }
                ref mut slot => {
                    *slot = Tree {
                        branches: vec![branch],
                        regex: None,
                    }.into()
                }
            }
        }
    }

    pub fn insert_regex(&mut self, mut regex: Regex, token: &'a Ident) {
        let first = regex.next().expect("#[regex] pattern musn't be empty");

        for byte in first {
            let branch = Branch(regex.clone(), token);

            match self.handlers[byte as usize] {
                Handler::Tree(ref mut tree) => {
                    // FIXME!
                    // tree.regex.insert(branch, "Two #[regex] patterns matching the same first byte are not allowed yet.");
                    tree.regex = Some(branch);
                },
                ref mut slot => *slot = Tree {
                    branches: Vec::new(),
                    regex: Some(branch),
                }.into()
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
