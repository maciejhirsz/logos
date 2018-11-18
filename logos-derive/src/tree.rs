use syn::Ident;
use std::{mem, fmt};
use std::cmp::Ordering;
use regex::{Regex, RepetitionFlag};
use util::OptionExt;

pub type Token<'a> = &'a Ident;

#[derive(Clone, Default, PartialEq)]
pub struct Branch<'a> {
    pub regex: Regex,
    pub then: Option<Box<Node<'a>>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ForkKind {
    Plain  = 0,
    Maybe  = 1,
    Repeat = 2,
}

impl Default for ForkKind {
    fn default() -> Self {
        ForkKind::Plain
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Fork<'a> {
    pub kind: ForkKind,
    pub arms: Vec<Branch<'a>>,
    pub then: Option<Box<Node<'a>>>,
}

#[derive(Clone, PartialEq)]
pub enum Node<'a> {
    Branch(Branch<'a>),
    Fork(Fork<'a>),
    Token(Token<'a>),
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Branch(branch) => branch.fmt(f),
            Node::Fork(fork) => fork.fmt(f),
            Node::Token(token) => write!(f, "TOKEN \"{}\"", token),
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

        f.debug_list().entries(self.arms.iter()).finish()?;

        if let Some(ref then) = self.then {
            f.write_str(" -> ")?;
            then.fmt(f)?;
        }

        Ok(())
    }
}

impl<'a> From<Token<'a>> for Node<'a> {
    fn from(token: Token<'a>) -> Self {
        Node::Token(token)
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

impl<'a> From<Fork<'a>> for Node<'a> {
    fn from(fork: Fork<'a>) -> Self {
        Node::Fork(fork)
    }
}

impl<'a> From<Fork<'a>> for Box<Node<'a>> {
    fn from(fork: Fork<'a>) -> Self {
        Node::Fork(fork).boxed()
    }
}

impl<'a> Branch<'a> {
    pub fn new(regex: Regex) -> Self {
        Branch {
            regex,
            then: None,
        }
    }

    pub fn compare(&self, other: &Branch<'a>) -> Ordering {
        other.regex.first().partial_cmp(self.regex.first()).unwrap_or_else(|| Ordering::Greater)
    }

    fn append_at_end(&mut self, then: &Node<'a>) {
        match self.then {
            Some(ref mut node) => node.append_at_end(then),
            None => {
                self.then = Some(then.clone().boxed());
            },
        }
    }

    pub fn insert_then(&mut self, other: Option<Box<Node<'a>>>) {
        if let Some(other) = other {
            self.then = match self.then.take() {
                Some(mut then) => {
                    then.insert(*other);

                    Some(then)
                },
                None => Some(other)
            };
        }
    }

    fn to_node(self) -> Option<Node<'a>> {
        if self.regex.len() == 0 {
            self.then.map(|node| *node)
        } else {
            Some(Node::Branch(self))
        }
    }
}

impl<'a> Fork<'a> {
    pub fn insert(&mut self, then: Node<'a>) {
        match then {
            Node::Branch(branch) => self.insert_branch(branch),
            Node::Token(_) => {
                self.collapse();
                self.then = Some(then.boxed());
            },
            Node::Fork(mut other)    => {
                if self.kind == other.kind && self.arms == other.arms {
                    self.insert_then(other.then.take());

                    return;
                }

                other.unwind();
                other.collapse();

                // self.unwind();
                // self.collapse();

                for branch in other.arms.into_iter() {
                    self.insert_branch(branch);
                }
            }
        }
    }

    pub fn insert_branch(&mut self, mut branch: Branch<'a>) {
        if branch.regex.len() == 0 {
            if let Some(then) = branch.then {
                return self.insert(*then);
            }
        }

        // Look for a branch that matches the same prefix
        for other in self.arms.iter_mut() {
            // We got a match!
            if let Some(regex) = branch.regex.match_split(&mut other.regex) {
                let old = mem::replace(other, Branch {
                    regex,
                    then: branch.to_node().map(Box::new),
                });

                other.insert_then(old.to_node().map(Box::new));

                return;
            }

            if let Some(prefix) = branch.regex.common_prefix(&other.regex) {
                let regex = Regex::from(prefix);

                let mut fork = Fork::default();
                let mut new = Branch {
                    regex,
                    then: Some(fork.into()),
                };

                if branch.regex.first() == new.regex.first() {
                    let mut other = other.clone();
                    let mut old = mem::replace(&mut branch, new);

                    old.regex.unshift();
                    other.regex.unshift();

                    branch.insert_then(old.to_node().map(Box::new));
                    branch.insert_then(other.to_node().map(Box::new));
                } else if other.regex.first() == new.regex.first() {
                    let mut branch = branch.clone();
                    let mut old = mem::replace(other, new);

                    old.regex.unshift();
                    branch.regex.unshift();

                    other.insert_then(old.to_node().map(Box::new));
                    other.insert_then(branch.to_node().map(Box::new));
                }
            }
        }

        // Sort arms of the fork, simple bytes in alphabetical order first, patterns last
        match self.arms.binary_search_by(|other| branch.compare(other)) {
            Ok(index) => {
                self.arms[index].then.as_mut().expect("Token conflict?").insert(branch.into());
            },
            Err(index) => {
                self.arms.insert(index, branch.into());
            },
        }
    }

    pub fn chain(&mut self, then: Node<'a>) {
        if let Some(ref current) = self.then {
            panic!("Trying to chain fork to {:#?}\n\nFork is already chained to {:#?}", then, current);
        }

        self.then = Some(then.boxed());
    }

    pub fn insert_then(&mut self, other: Option<Box<Node<'a>>>) {
        if let Some(other) = other {
            self.then = match self.then.take() {
                Some(mut then) => {
                    then.insert(*other);

                    Some(then)
                },
                None => Some(other)
            };
        }
    }

    fn unwind(&mut self) {
        if self.kind != ForkKind::Repeat {
            return;
        }

        let repeat = self.clone();
        let repeat = mem::replace(self, repeat);

        self.kind = ForkKind::Plain;

        // Also do this?
        // self.insert(repeat.clone());

        self.then = Some(Node::from(repeat).boxed());
    }

    fn collapse(&mut self) {
        let then = match self.then.take() {
            None => return,
            Some(node) => node,
        };

        for branch in self.arms.iter_mut() {
            branch.append_at_end(&then);
        }
    }

    fn append_at_end(&mut self, then: &Node<'a>) {
        match self.then {
            Some(ref mut node) => node.append_at_end(then),
            None => {
                self.then = Some(then.clone().boxed());
            },
        }

        for branch in self.arms.iter_mut() {
            branch.append_at_end(then)
        }
    }
}

impl<'a> Node<'a> {
    pub fn new(regex: Regex, token: Token<'a>) -> Self {
        if regex.len() == 0 {
            Node::Token(token)
        } else {
            Node::Branch(Branch {
                regex,
                then: Some(Node::from(token).boxed()),
            })
        }
    }

    fn to_mut_fork(&mut self) -> &mut Fork<'a> {
        let fork = match self {
            Node::Fork(fork) => return fork,
            Node::Branch(branch) => {
                let arm = Branch {
                    regex: branch.regex.clone(),
                    then: None,
                };

                Fork {
                    kind: ForkKind::Plain,
                    arms: vec![arm],
                    then: branch.then.clone(),
                }
            },
            Node::Token(_) => {
                Fork {
                    kind: ForkKind::Plain,
                    arms: vec![],
                    then: Some(self.clone().boxed()),
                }
            }
        };

        mem::replace(self, Node::Fork(fork));

        if let Node::Fork(fork) = self {
            fork
        } else {
            unreachable!()
        }
    }

    pub fn insert(&mut self, then: Node<'a>) {
        self.to_mut_fork().insert(then);
    }

    pub fn make_repeat(&mut self, flag: RepetitionFlag) {
        use self::RepetitionFlag::*;

        if let Node::Branch(branch) = self {
            if flag == OneOrMore {
                let mut next = Node::from(branch.clone());

                next.to_mut_fork().kind = ForkKind::Repeat;

                return branch.then = Some(next.boxed());
            }
        }

        let fork = self.to_mut_fork();

        match flag {
            ZeroOrOne => fork.kind = ForkKind::Maybe,
            ZeroOrMore => fork.kind = ForkKind::Repeat,
            OneOrMore => {
                let mut next: Fork = fork.clone();

                next.kind = ForkKind::Repeat;

                fork.then = Some(next.into());
            }
        }
    }

    pub fn chain(&mut self, then: Node<'a>) {
        match self {
            Node::Branch(branch) => branch.then = Some(then.boxed()),
            Node::Fork(fork) => fork.then = Some(then.boxed()),
            Node::Token(_) => panic!("Cannot chain on `Token` nodes"),
        }
    }

    /// Tests whether the branch produces a token on all leaves without any tests.
    pub fn exhaustive(&self) -> bool {
        match self {
            Node::Token(_) => true,
            Node::Branch(branch) => {
                branch.regex.len() == 1
                    && branch.then.as_ref().map(|then| then.exhaustive()).unwrap_or(false)
            },
            Node::Fork(fork) => {
                fork.then.is_some()
                    && fork.then.as_ref().map(|then| then.exhaustive()).unwrap_or(false)
                    && (fork.kind == ForkKind::Plain || fork.arms.len() == 1)
                    && fork.arms.iter().all(|branch| {
                        branch.regex.len() == 1
                            && branch.then.as_ref().map(|then| then.exhaustive()).unwrap_or(false)
                    })
            },
        }
    }

    pub fn fallback(&mut self) -> Option<Branch<'a>> {
        match self {
            Node::Fork(fork) => {
                if fork.kind != ForkKind::Repeat {
                    return None;
                }

                // This is a bit weird, but it basically checks if the fork
                // has one and only one branch that is heavy and if so, it
                // removes that branch and returns it.
                //
                // FIXME: This should check if all other branches in the tree
                //        are specializations of that regex
                let mut index = None;

                for (idx, branch) in fork.arms.iter().enumerate() {
                    if branch.regex.first().weight() > 1 {
                        // Make sure we only get one
                        if index.is_some() {
                            return None;
                        }

                        index = Some(idx);
                    }
                }

                index.map(|idx| fork.arms.remove(idx))
            }
            _ => None,
        }
    }

    /// Get all tokens in this tree
    pub fn get_tokens(&self, vec: &mut Vec<&'a Ident>) {
        fn insert<'a>(vec: &mut Vec<&'a Ident>, token: &'a Ident) {
            if let Err(index) = vec.binary_search(&token) {
                vec.insert(index, token);
            }
        }

        match self {
            Node::Token(token) => insert(vec, token),
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

    fn append_at_end(&mut self, then: &Node<'a>) {
        match self {
            Node::Branch(branch) => branch.append_at_end(then),
            Node::Fork(fork) => fork.append_at_end(then),
            Node::Token(_) => {},
        }
    }

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}
