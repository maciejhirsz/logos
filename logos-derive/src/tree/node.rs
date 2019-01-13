use std::{mem, fmt};

use super::{Branch, Leaf, Fork, Token};
use super::ForkKind::*;

use crate::regex::{Regex, Pattern, RepetitionFlag};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Node<'a> {
    Branch(Branch<'a>),
    Fork(Fork<'a>),
    Leaf(Leaf<'a>),
}

impl<'a> Node<'a> {
    pub fn new(regex: Regex) -> Self {
        if regex.len() == 0 {
            panic!("Internal error: Trying to create a Node out of an empty Regex");
        }

        Node::Branch(Branch::new(regex))
    }

    pub fn leaf(mut self, leaf: Leaf<'a>) -> Self {
        self.chain(&Node::Leaf(leaf));

        self
    }

    pub fn is_leaf(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            _ => false,
        }
    }

    pub fn as_mut_fork(&mut self) -> Option<&mut Fork<'a>> {
        match self {
            Node::Fork(fork) => Some(fork),
            _ => None,
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

    pub fn find_fallback(&mut self, other: &mut Fork<'a>) -> Option<Fork<'a>> {
        if other.kind != Repeat {
            return None;
        }

        let patterns = match self {
            Node::Fork(fork) => fork.arms.iter().map(|arm| arm.regex.first()).collect(),
            Node::Branch(branch) => vec![branch.regex.first()],
            Node::Leaf(_) => return None,
        };

        let mut remove = Vec::new();
        let mut fallback: Option<Fork<'a>> = None;

        for a in patterns {
            for (index, arm) in other.arms.iter().enumerate() {
                let b = arm.regex.first();

                if b != a && b.contains(a) {
                    if let Some(ref mut fallback) = fallback {
                        let mut tokens = Vec::new();

                        other.then.iter().chain(fallback.then.iter())
                            .for_each(|node| node.get_tokens(&mut tokens));

                        let tokens = tokens.into_iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");

                        panic!("Failed to disambiguate: {}", tokens);
                    } else {
                        fallback =
                            Some(Fork::new(Repeat)
                                .arm(arm.clone())
                                .then(*other.then.clone().unwrap()));
                    }

                    remove.push(index);
                }
            }

            for index in remove.drain(..).rev() {
                other.arms.remove(index);
            }
        }

        fallback
    }

    /// Get all tokens in this tree
    pub fn get_tokens(&self, vec: &mut Vec<Token<'a>>) {
        fn insert<'a>(vec: &mut Vec<Token<'a>>, token: Token<'a>) {
            if let Err(index) = vec.binary_search(&token) {
                vec.insert(index, token);
            }
        }

        match self {
            Node::Leaf(Leaf::Token { token, .. }) => insert(vec, token),
            Node::Leaf(Leaf::Trivia) => {},
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
