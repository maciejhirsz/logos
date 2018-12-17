use std::rc::Rc;

use crate::tree::{Node, Branch, Leaf};
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
    pub leaf: Leaf<'a>,
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

impl<'a> Handlers<'a> {
    pub fn new() -> Self {
        let mut handlers = vec![Handler::Error; 256];

        handlers[0..33].iter_mut().for_each(|slot| *slot = Handler::Whitespace);

        Handlers {
            handlers
        }
    }

    pub fn insert(&mut self, mut branch: Branch<'a>) {
        let pattern = branch.regex.unshift();

        let tree = Rc::new(Tree {
            node: Node::from(branch),
            fallback: None,
        });

        let fallback = tree.node.fallback();

        for byte in pattern.to_bytes(&mut [0; 256]) {
            match self.handlers[*byte as usize] {
                Handler::Tree(ref mut tree) => {
                    let tree = Rc::make_mut(tree);

                    if let Some(ref fallback) = fallback {
                        if tree.node.matches(&fallback.boundary) {
                            tree.fallback = Some(fallback.clone());
                            continue;
                        }
                    }

                    tree.node.insert(tree.node.clone());
                },
                ref mut slot => *slot = Handler::Tree(tree.clone()),
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
