use std::{mem, fmt};

use super::{Branch, Leaf, Fork, Token};
use super::ForkKind::*;

use crate::handlers::Fallback;
use crate::regex::{Regex, Pattern, RepetitionFlag};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Node<'a> {
    Branch(Branch<'a>),
    Fork(Fork<'a>),
    Leaf(Leaf<'a>),
}

impl<'a> Node<'a> {
    pub fn new(regex: Regex, leaf: Leaf<'a>) -> Self {
        if regex.len() == 0 {
            Node::Leaf(leaf)
        } else {
            Node::Branch(Branch::new(regex).then(leaf))
        }
    }

    pub fn is_leaf(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            _ => false,
        }
    }

    fn to_mut_fork(&mut self) -> &mut Fork<'a> {
        match self {
            Node::Fork(fork) => return fork,
            Node::Branch(ref mut branch) => {
                let branch = mem::replace(branch, Branch::default());

                *self = Node::Fork(Fork::new(Plain).arm(branch))
            },
            Node::Leaf(leaf) => {
                *self = Node::Fork(Fork::new(Maybe).then(leaf.take()))
            }
        };

        if let Node::Fork(fork) = self {
            fork
        } else {
            panic!("Internal Error: Failed to convert node to a Fork")
        }
    }

    pub fn make_maybe_fork(&mut self) {
        match self {
            Node::Fork(fork) => {
                assert!(fork.kind != Repeat);

                return fork.kind = Maybe;
            },
            Node::Branch(ref mut branch) => {
                let branch = mem::replace(branch, Branch::default());

                *self = Node::Fork(Fork::new(Maybe).arm(branch));
            },
            Node::Leaf(_) => {},
        }
    }

    pub fn insert<Then>(&mut self, then: Then)
    where
        Then: Into<Node<'a>>,
    {
        let then = then.into();

        if self.is_leaf() && then.is_leaf() {
            return;
        }

        let fork = self.to_mut_fork();

        fork.insert(then);

        if fork.arms.len() == 0 {
            if let Some(then) = fork.then.take() {
                *self = *then;
            }
        }
    }

    pub fn make_repeat(&mut self, flag: RepetitionFlag) {
        use self::RepetitionFlag::*;

        if let Node::Branch(branch) = self {
            if flag == OneOrMore {
                let mut next = Node::from(branch.clone());

                next.to_mut_fork().kind = Repeat;

                return branch.then = Some(next.boxed());
            }
        }

        let fork = self.to_mut_fork();

        match flag {
            ZeroOrOne => fork.kind = Maybe,
            ZeroOrMore => fork.kind = Repeat,
            OneOrMore => {
                let mut next: Fork = fork.clone();

                next.kind = Repeat;

                fork.insert_then(Some(next.into()));
            }
        }
    }

    /// Checks if all branches in the node match a specific pattern
    pub fn matches(&self, pattern: &Pattern) -> bool {
        match self {
            Node::Branch(branch) => branch.matches(pattern),
            Node::Fork(fork) => {
                fork.arms.iter().all(|arm| arm.matches(pattern))
                    && fork.then.as_ref().map(|then| then.matches(pattern)).unwrap_or(true)
            },
            Node::Leaf(_) => true,
        }
    }

    pub fn fallback(&self) -> Option<Fallback<'a>> {
        match self {
            Node::Fork(fork) => {
                let arm = &fork.arms[0];
                let leaf = match &fork.then {
                    Some(node) => match **node {
                        Node::Leaf(ref leaf) => leaf,
                        _ => return None,
                    },
                    _ => return None,
                };

                if fork.kind == Repeat
                    && fork.arms.len() == 1
                    && arm.regex.len() == 1
                    && arm.then.is_none()
                {
                    Some(Fallback {
                        boundary: arm.regex.first().clone(),
                        leaf: leaf.clone(),
                    })
                } else {
                    None
                }
            },
            _ => None
        }
    }

    /// Get all tokens in this tree
    pub fn get_tokens(&self, vec: &mut Vec<Token<'a>>) {
        fn insert<'a>(vec: &mut Vec<Token<'a>>, token: Token<'a>) {
            if let Err(index) = vec.binary_search(&token) {
                vec.insert(index, token);
            }
        }

        match self {
            Node::Leaf(leaf) => insert(vec, leaf.token),
            Node::Branch(branch) => {
                if let Some(ref then) = branch.then {
                    then.get_tokens(vec);
                }
            },
            Node::Fork(fork) => {
                for branch in fork.arms.iter() {
                    if let Some(ref then) = branch.then {
                        then.get_tokens(vec);
                    }
                }
                if let Some(ref then) = fork.then {
                    then.get_tokens(vec);
                }
            }
        }
    }

    pub fn chain(&mut self, then: &Node<'a>) {
        match self {
            Node::Branch(branch) => branch.chain(then),
            Node::Fork(fork) => fork.chain(then),
            Node::Leaf(_) => {},
        }
    }

    pub fn is_token(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            _ => false,
        }
    }

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    pub fn min_bytes(&self) -> usize {
        match self {
            Node::Fork(fork) if fork.kind == Plain => fork.min_bytes(),
            Node::Fork(_) => 0,
            Node::Branch(branch) => branch.min_bytes(),
            Node::Leaf(_) => 0,
        }
    }

    pub fn pack(&mut self) {
        match self {
            Node::Fork(fork) => {
                fork.pack();

                if fork.kind == Plain
                    && fork.arms.len() == 1
                    && (fork.then.is_none() || fork.arms[0].then.is_none())
                {
                    let mut branch = fork.arms.remove(0);
                    branch.then = branch.then.or(fork.then.take());
                    branch.pack();

                    *self = Node::Branch(branch);
                }
            },
            Node::Branch(branch) => branch.pack(),
            Node::Leaf(_) => {}
        }
    }
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Branch(branch) => branch.fmt(f),
            Node::Fork(fork) => fork.fmt(f),
            Node::Leaf(leaf) => write!(f, "TOKEN \"{:?}\"", leaf),
        }
    }
}
