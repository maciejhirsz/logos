use std::rc::Rc;

use crate::tree::{Node, Branch};

#[derive(Debug, Clone)]
pub enum Handler<'a> {
    Error,
    Whitespace,
    Tree(Rc<Node<'a>>),
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
        let bytes = branch.regex.unshift().to_bytes();

        let node = Rc::new(Node::from(branch));

        for byte in bytes {
            match self.handlers[byte as usize] {
                Handler::Tree(ref mut root) => Rc::make_mut(root).insert((*node).clone()),
                ref mut slot => *slot = Handler::Tree(node.clone()),
            }
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Handler<'a>> {
        self.handlers.into_iter()
    }
}
