use std::fmt;
use std::cmp::Ordering;

use super::Node;
use super::ForkKind::*;
use crate::regex::{Regex, Pattern};

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Branch<'a> {
    pub regex: Regex,
    pub then: Option<Box<Node<'a>>>,
}

impl<'a> Branch<'a> {
    pub fn new<R>(regex: R) -> Self
    where
        R: Into<Regex>,
    {
        Branch {
            regex: regex.into(),
            then: None,
        }
    }

    pub fn then<Then>(self, then: Then) -> Self
    where
        Then: Into<Node<'a>>
    {
        Branch {
            regex: self.regex,
            then: Some(then.into().boxed())
        }
    }

    pub fn compare(&self, other: &Branch<'a>) -> Ordering {
        other.regex.first().partial_cmp(self.regex.first()).unwrap_or_else(|| Ordering::Greater)
    }

    pub fn chain(&mut self, then: &Node<'a>) {
        match self.then {
            Some(ref mut node) => node.chain(then),
            None => {
                self.then = Some(then.clone().boxed());
            },
        }
    }

    pub fn insert_then<Other>(&mut self, other: Other)
    where
        Other: Into<Option<Box<Node<'a>>>>,
    {
        let other = other.into();

        match self.then {
            Some(ref mut node) => {
                match other {
                    Some(other) => node.insert(*other),
                    None => node.make_maybe_fork(),
                }
            }
            ref mut then => *then = other,
        }
    }

    pub fn to_node(self) -> Option<Node<'a>> {
        if self.regex.len() == 0 {
            self.then.map(|node| *node)
        } else {
            Some(Node::Branch(self))
        }
    }

    pub fn is_finite(&self) -> bool {
        match self.then {
            Some(ref node) => match **node {
                Node::Fork(ref fork) => fork.kind == Plain,
                _ => true,
            },
            None => false,
        }
    }

    pub fn matches(&self, pattern: &Pattern) -> bool {
        self.regex
            .patterns()
            .iter()
            .all(|pat| pattern.contains(pat))
    }

    pub fn min_bytes(&self) -> usize {
        self.regex.len() + self.then.as_ref().map(|node| node.min_bytes()).unwrap_or(0)
    }

    pub fn pack(&mut self) {
        if let Some(ref mut then) = self.then {
            then.pack();

            match &mut **then {
                Node::Branch(branch) => {
                    if let Some(next) = &mut branch.then {
                        next.pack();
                    }

                    self.regex.extend(branch.regex.patterns());
                    self.then = branch.then.take();
                },
                Node::Fork(fork) => fork.pack(),
                Node::Leaf(_) => {},
            }
        }
    }
}

impl<'a> fmt::Debug for Branch<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.regex.fmt(f)?;

        if let Some(ref then) = self.then {
            f.write_str(" -> ")?;
            then.fmt(f)?;
        }

        Ok(())
    }
}

impl<'a> From<Branch<'a>> for Node<'a> {
    fn from(branch: Branch<'a>) -> Self {
        if branch.regex.len() == 0 {
            *branch.then.expect("Cannot convert an empty branch to a Node!")
        } else {
            Node::Branch(branch)
        }
    }
}

impl<'a> From<Branch<'a>> for Option<Box<Node<'a>>> {
    fn from(branch: Branch<'a>) -> Self {
        branch.to_node().map(Box::new)
    }
}
