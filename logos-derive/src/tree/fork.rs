use std::{mem, fmt};
use rustc_hash::FxHashMap as HashMap;

use super::{Node, Branch};
use crate::regex::Pattern;

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Fork<'a> {
    pub kind: ForkKind,
    pub arms: Vec<Branch<'a>>,
    pub then: Option<Box<Node<'a>>>,
}


#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ForkKind {
    Plain  = 0,
    Maybe  = 1,
    Repeat = 2,
}

pub use self::ForkKind::*;

impl Default for ForkKind {
    fn default() -> Self {
        ForkKind::Plain
    }
}

impl<'a> Fork<'a> {
    pub fn new(kind: ForkKind) -> Self {
        Fork {
            kind,
            arms: Vec::new(),
            then: None,
        }
    }

    pub fn arm(mut self, arm: Branch<'a>) -> Self {
        self.arms.push(arm);
        self
    }

    pub fn then<Then>(mut self, then: Then) -> Self
    where
        Then: Into<Node<'a>>
    {
        self.then = Some(then.into().boxed());
        self
    }

    pub fn insert<Then>(&mut self, then: Then)
    where
        Then: Into<Node<'a>>,
    {
        let then = then.into();

        match then {
            Node::Branch(branch) => {
                // If possible, we unwind repeat forks and collapse maybe forks.
                self.unwind();
                self.collapse();

                self.insert_branch(branch);
            },
            Node::Leaf(leaf) => {
                if self.then.is_none() {
                    assert!(
                        self.kind == Plain,
                        "Internal Error: Invalid fork construction: {:#?}", self
                    );

                    self.kind = Maybe;
                    self.then = Some(Node::Leaf(leaf).boxed());
                } else {
                    self.unwind();
                    self.collapse();

                    assert!(
                        self.kind != Plain,
                        "Internal Error: Invalid fork construction: {:#?}", self
                    );

                    self.kind = Maybe;
                    self.then = Some(Node::Leaf(leaf).boxed());
                }
            },
            Node::Fork(mut other) => {
                if self.kind == other.kind && self.arms == other.arms {
                    self.insert_then(other.then.take());

                    return;
                }

                // If possible, we unwind repeat forks and collapse maybe forks.
                self.unwind();
                self.collapse();
                other.unwind();
                other.collapse();

                if other.kind > self.kind {
                    self.kind = other.kind;
                }

                self.insert_then(other.then.take());

                for branch in other.arms.into_iter() {
                    self.insert_branch(branch);
                }
            }
        }
    }

    pub fn insert_branch(&mut self, mut branch: Branch<'a>) {
        if branch.regex.len() == 0 {
            return self.insert_then(branch.then);
        }

        // Looking for intersection prefixes, that is: A ≠ B & (A ⊂ B | B ⊂ A)
        for other in self.arms.iter_mut() {
            if let Some(prefix) = branch.regex.common_prefix(&other.regex) {
                let mut intersection = Branch::new(prefix);

                let mut a = branch.clone();
                let mut b = other.clone();

                a.regex.unshift();
                b.regex.unshift();

                intersection.fallback = a.fallback.take().or(b.fallback.take());
                intersection.insert_then(a);
                intersection.insert_then(b);

                if intersection.regex.first() == branch.regex.first() {
                    other.regex.first_mut().subtract(intersection.regex.first());
                    branch = intersection;
                } else {
                    branch.regex.first_mut().subtract(intersection.regex.first());
                    *other = intersection;
                }
            }
        }

        // Look for a branch that matches the same prefix
        for other in self.arms.iter_mut() {
            // We got a match!
            if let Some(regex) = branch.regex.match_split(&mut other.regex) {
                let mut old = mem::replace(other, Branch::new(regex));

                other.fallback = old.fallback.take();

                let a = branch.to_node().map(Box::new);
                let b = old.to_node().map(Box::new);

                let maybe_fork = a.is_none() || b.is_none();

                other.insert_then(a);
                other.insert_then(b);

                if maybe_fork {
                    if let Some(ref mut then) = other.then {
                        then.make_maybe_fork();
                    }
                }

                return;
            }
        }

        self.sorted_insert_arm(branch);
    }

    fn sorted_insert_arm(&mut self, branch: Branch<'a>) {
        // Sort arms of the fork, simple bytes in alphabetical order first, patterns last
        match self.arms.binary_search_by(|other| branch.compare(other)) {
            Ok(index) => {
                self.arms[index].insert_then(branch);
            },
            Err(index) => {
                self.arms.insert(index, branch);
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
            },
            None => {
                if other.is_some() {
                    assert!(
                        self.kind != Repeat,
                        "Internal Error: Invalid fork construction"
                    );

                    self.kind = Maybe;
                    self.then = other;
                }
            },
        }
    }

    /// Unwinds a Repeat fork into a Maybe fork
    pub fn unwind(&mut self) {
        if self.kind != Repeat {
            return;
        }

        let repeat = Node::from(self.clone());

        for branch in self.arms.iter_mut() {
            branch.chain(&repeat);
        }

        if let Some(mut then) = self.then.take() {
            match *then {
                Node::Fork(ref mut fork) if fork.kind == Plain => {
                    for branch in fork.arms.drain(..) {
                        self.insert_branch(branch);
                    }

                    return self.kind = Plain;
                },
                Node::Branch(ref mut branch) => {
                    self.insert_branch(branch.clone());

                    return self.kind = Plain;
                },
                _ => self.then = Some(then),
            }
        }

        self.kind = Maybe;
    }

    // Attempts to collapse a Maybe fork into a Plain fork.
    // If `then` on this fork is a `Token`, or if it isn't
    // set, then it will remain a Maybe fork.
    pub fn collapse(&mut self) {
        if self.kind != Maybe {
            return;
        }

        let then = match self.then.take() {
            None => return,
            Some(node) => node,
        };

        for branch in self.arms.iter_mut() {
            branch.chain(&*then);
        }

        if then.is_token() {
            self.then = Some(then);
        } else {
            self.kind = Plain;
            self.insert(*then);
        }
    }

    pub fn chain(&mut self, then: &Node<'a>) {
        if self.kind == Plain {
            for branch in self.arms.iter_mut() {
                branch.chain(then)
            }
        } else {
            match self.then {
                Some(ref mut node) => node.chain(then),
                None => {
                    self.then = Some(then.clone().boxed());
                },
            }
        }
    }

    /// Minimum amount of bytes that will satisfy this Fork
    pub fn min_bytes(&self) -> usize {
        self.arms
            .iter()
            .map(|arm| arm.min_bytes())
            .min()
            .unwrap_or(0)
    }

    pub fn pack(&mut self) {
        self.collapse();

        if let Some(then) = &mut self.then {
            then.pack();
        }

        self.arms.iter_mut().for_each(Branch::pack);

        if self.arms.len() > 1 {
            let mut scan: HashMap<(&Option<Box<Node>>, &[Pattern]), usize> = HashMap::default();
            let mut remove = Vec::new();

            for (index, arm) in self.arms.iter_mut().enumerate() {
                let first = arm.regex.first();
                let tail = &arm.regex.patterns()[1..];

                let retain = *scan.entry((&arm.then, tail)).or_insert(index);

                if retain != index {
                    remove.push((retain, index, first.clone()));
                }
            }

            for (retain, index, pattern) in remove.into_iter().rev() {
                self.arms[retain].regex.first_mut().combine(&pattern);
                self.arms.remove(index);
            }
        }

        if self.kind == Maybe && self.arms.len() == 1 && self.arms[0].then == self.then {
            self.arms[0].then = None;
        }
    }
}

impl fmt::Debug for ForkKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ForkKind::Plain => Ok(()),
            ForkKind::Maybe => f.write_str("MAYBE "),
            ForkKind::Repeat => f.write_str("REPEAT "),
        }
    }
}

impl<'a> fmt::Debug for Fork<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.kind.fmt(f)?;

        if self.arms.len() == 1 && self.arms[0].then.is_none() {
            f.write_str("[")?;
            self.arms[0].fmt(f)?;
            f.write_str("]")?;
        } else {
            f.debug_list().entries(self.arms.iter()).finish()?;
        }

        if let Some(ref then) = self.then {
            f.write_str(" -> ")?;
            then.fmt(f)?;
        }

        Ok(())
    }
}

impl<'a> From<Fork<'a>> for Node<'a> {
    fn from(fork: Fork<'a>) -> Self {
        if fork.arms.is_empty() {
            if let Some(then) = fork.then {
                return *then;
            }
        }

        Node::Fork(fork)
    }
}

impl<'a> From<Fork<'a>> for Box<Node<'a>> {
    fn from(fork: Fork<'a>) -> Self {
        Node::from(fork).boxed()
    }
}
