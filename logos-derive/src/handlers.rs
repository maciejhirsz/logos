use std::rc::Rc;

use crate::tree::{Node, Branch, Fork};
use crate::regex::Pattern;

#[derive(Debug, Clone)]
pub enum Handler<'a> {
    Error,
    Whitespace,
    Tree(Rc<Tree<'a>>),
}

#[derive(Debug, Clone)]
pub struct Fallback<'a> {
    pub boundary: Pattern,
    pub fork: Fork<'a>,
}

#[derive(Debug, Clone)]
pub struct Tree<'a> {
    pub node: Node<'a>,
    pub fallback: Option<Fallback<'a>>,
}

#[derive(Debug)]
pub struct Handlers<'a> {
    handlers: Vec<Handler<'a>>,
}

pub enum Trivia {
    Pattern(Pattern),
    Default,
    None,
}

impl<'a> Handlers<'a> {
    pub fn new(trivia: Trivia) -> Self {
        let mut handlers = vec![Handler::Error; 256];

        match trivia {
            Trivia::Pattern(pat) => {
                for byte in pat.bytes() {
                    handlers[byte as usize] = Handler::Whitespace;
                }
            },
            Trivia::Default => {
                handlers[0..33].iter_mut().for_each(|slot| *slot = Handler::Whitespace);
            },
            Trivia::None => {}
        }

        Handlers {
            handlers
        }
    }

    pub fn insert(&mut self, mut branch: Branch<'a>) {
        let pattern = branch.regex.unshift();
        let fallback = branch.fallback.take().map(|fork| {
            let boundary = fork.arms[0].regex.first().clone();

            Fallback {
                boundary,
                fork,
            }
        });

        let tree = Rc::new(Tree {
            node: Node::from(branch),
            fallback,
        });

        for byte in pattern.bytes() {
            self.handlers[byte as usize] = Handler::Tree(tree.clone());
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
