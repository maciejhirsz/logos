use std::{mem, fmt};
use std::cmp::Ordering;
use regex::{Regex, RepetitionFlag};
use util::OptionExt;

pub type Token<'a> = &'a ::syn::Ident;

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

    fn chain(&mut self, then: &Node<'a>) {
        match self.then {
            Some(ref mut node) => node.chain(then),
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
            Node::Token(token) => {
                if self.then.is_none() {
                    // assert!(
                    //     self.kind == ForkKind::Plain,
                    //     "Internal Error: Invalid fork construction: {:#?}", self
                    // );

                    self.kind = ForkKind::Maybe;
                    self.then = Some(Node::Token(token).boxed());
                } else {
                    self.collapse();

                    assert!(
                        self.kind != ForkKind::Plain,
                        "Internal Error: Invalid fork construction: {:#?}", self
                    );

                    self.kind = ForkKind::Maybe;
                    self.then = Some(Node::Token(token).boxed());
                }

                // FIXME: look up through all tokens produced by `self.then`,
                //        if they point at a token different from `token`,
                //        panic with an error about conflicting definitions!
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

        if self.kind == ForkKind::Plain {
            // Looking for intersection prefixes, that is: A ≠ B & (A ⊂ B | B ⊂ A)
            for other in self.arms.iter_mut() {
                if let Some(prefix) = branch.regex.common_prefix(&other.regex) {
                    let mut intersection = Branch::new(Regex::from(prefix));

                    let mut a = branch.clone();
                    let mut b = other.clone();

                    a.regex.unshift();
                    b.regex.unshift();

                    intersection.insert_then(a.to_node().map(Box::new));
                    intersection.insert_then(b.to_node().map(Box::new));

                    if intersection.regex.first() == branch.regex.first() {
                        branch = intersection;
                    } else {
                        mem::swap(other, &mut intersection);
                        // byproducts.push(intersection);
                    }
                }
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
        }

        // Sort arms of the fork, simple bytes in alphabetical order first, patterns last
        match self.arms.binary_search_by(|other| branch.compare(other)) {
            Ok(index) => {
                // self.arms[index].then.as_mut().expect("Token conflict?").insert(branch.into());
                self.arms[index].insert_then(branch.to_node().map(Box::new));
            },
            Err(index) => {
                self.arms.insert(index, branch.into());
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
                None => {
                    assert!(
                        self.kind == ForkKind::Plain,
                        "Internal Error: Invalid fork construction: {:#?}", self
                    );

                    self.kind = ForkKind::Maybe;

                    Some(other)
                },
            };
        }
    }

    /// Unwinds a Repeat fork into a Maybe fork
    fn unwind(&mut self) {
        // let before = self.clone();

        if self.kind != ForkKind::Repeat {
            return;
        }

        let repeat = Node::Fork(self.clone());

        for branch in self.arms.iter_mut() {
            branch.chain(&repeat);
        }

        self.kind = ForkKind::Maybe;
    }

    // Attempts to collapse a Maybe fork into a Plain fork.
    // If `then` on this fork is a `Token`, then it will
    // remain a Maybe fork.
    fn collapse(&mut self) {
        if self.kind != ForkKind::Maybe {
            return;
        }

        let then = match self.then.take() {
            None => panic!("Invalid fork construction: {:#?}", self),
            Some(node) => node,
        };

        for branch in self.arms.iter_mut() {
            branch.chain(&*then);
        }

        if then.is_token() {
            self.then = Some(then);
        } else {
            self.kind = ForkKind::Plain;
        }
    }

    fn chain(&mut self, then: &Node<'a>) {
        if self.kind == ForkKind::Plain {
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
                // FIXME: a memswap of the branch could save an allocation here
                Fork {
                    kind: ForkKind::Plain,
                    arms: vec![branch.clone()],
                    then: None,
                }
            },
            Node::Token(_) => {
                Fork {
                    kind: ForkKind::Maybe,
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

                fork.insert_then(Some(next.into()));
            }
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
    pub fn get_tokens(&self, vec: &mut Vec<Token<'a>>) {
        fn insert<'a>(vec: &mut Vec<Token<'a>>, token: Token<'a>) {
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

    pub fn chain(&mut self, then: &Node<'a>) {
        match self {
            Node::Branch(branch) => branch.chain(then),
            Node::Fork(fork) => fork.chain(then),
            Node::Token(_) => {},
        }
    }

    fn is_token(&self) -> bool {
        match self {
            Node::Token(_) => true,
            _ => false,
        }
    }

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::Ident;

    fn token(mock: &str) -> Ident {
        use ::proc_macro2::Span;

        Ident::new(mock, Span::call_site())
    }

    fn branch<'a>(regex: &str, token: Token<'a>) -> Branch<'a> {
        Branch {
            regex: Regex::sequence(regex),
            then: Some(Node::Token(token).boxed()),
        }
    }

    #[test]
    fn branch_to_node() {
        let regex = Regex::sequence("abc");
        let branch = Branch::new(regex.clone());

        assert_eq!(branch.to_node(), Some(Node::Branch(Branch {
            regex: regex,
            then: None
        })));
    }

    #[test]
    fn empty_branch_to_node() {
        let branch = Branch::new(Regex::default());

        assert_eq!(branch.to_node(), None);
    }

    #[test]
    fn empty_branch_with_then_to_node() {
        let token = token("mock");
        let branch = Branch {
            regex: Regex::default(),
            then: Some(Node::Token(&token).boxed()),
        };

        assert_eq!(branch.to_node(), Some(Node::Token(&token)));
    }

    #[test]
    fn insert_branch_into_branch() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let branch_a = branch("abc", &token_a);
        let branch_b = branch("def", &token_b);

        let mut parent = branch("abc", &token_a).to_node().unwrap();
        let child = branch("def", &token_b).to_node().unwrap();

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch_a, branch_b],
            then: None,
        }));
    }

    #[test]
    fn insert_a_token_into_a_branch() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = branch("abc", &token_a).to_node().unwrap();
        let child = Node::Token(&token_b);

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("abc", &token_a)],
            then: Some(Node::Token(&token_b).boxed()),
        }));
    }

    #[test]
    fn insert_a_branch_into_a_token() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Token(&token_a);
        let child = branch("xyz", &token_b).to_node().unwrap();

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("xyz", &token_b)],
            then: Some(Node::Token(&token_a).boxed()),
        }));
    }

    #[test]
    fn insert_a_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("abc", &token_a)],
            then: None,
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("def", &token_b)],
            then: None,
        });

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![
                branch("abc", &token_a),
                branch("def", &token_b),
            ],
            then: None,
        }));
    }

    #[test]
    fn insert_a_maybe_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");
        let token_x = token("XYZ");

        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("abc", &token_a)],
            then: None,
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("def", &token_b)],
            then: Some(Node::Token(&token_x).boxed()),
        });

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", &token_a),
                branch("def", &token_b),
            ],
            then: Some(Node::Token(&token_x).boxed()),
        }));
    }

    #[test]
    fn insert_a_fork_into_a_maybe_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");
        let token_x = token("XYZ");

        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("abc", &token_a)],
            then: Some(Node::Token(&token_x).boxed()),
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("def", &token_b)],
            then: None,
        });

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", &token_a),
                branch("def", &token_b),
            ],
            then: Some(Node::Token(&token_x).boxed()),
        }));
    }

    #[test]
    fn collapsing_a_fork() {
        let token_a = token("ABC");

        let mut fork = Fork {
            kind: ForkKind::Maybe,
            arms: vec![Branch::new(Regex::sequence("abc"))],
            then: Some(Node::Token(&token_a).boxed()),
        };

        let expected = Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("abc", &token_a)],
            then: Some(Node::Token(&token_a).boxed()),
        };

        fork.collapse();

        assert!(fork == expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", fork, expected);
    }

    #[test]
    fn unwinding_a_fork() {
        let token_a = token("ABC");

        let mut fork = Fork {
            kind: ForkKind::Repeat,
            arms: vec![Branch::new(Regex::sequence("abc"))],
            then: Some(Node::Token(&token_a).boxed()),
        };

        let expected = Fork {
            kind: ForkKind::Maybe,
            arms: vec![Branch {
                regex: Regex::sequence("abc"),
                then: Some(Node::Fork(fork.clone()).boxed()),
            }],
            then: Some(Node::Token(&token_a).boxed()),
        };

        fork.unwind();
        // fork.collapse();

        assert!(fork == expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", fork, expected);
    }

    #[test]
    fn insert_a_repeat_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("abc", &token_a)],
            then: None,
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Repeat,
            arms: vec![Branch::new(Regex::sequence("def"))],
            then: Some(Node::Token(&token_b).boxed()),
        });

        let expected = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", &token_a),
                Branch {
                    regex: Regex::sequence("def"),
                    then: Some(child.clone().boxed()),
                }
            ],
            then: Some(Node::Token(&token_b).boxed()),
        });

        parent.insert(child);

        assert!(parent == expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", parent, expected);
    }

    #[test]
    fn insert_a_fork_into_a_repeat_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");


        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Repeat,
            arms: vec![Branch::new(Regex::sequence("def"))],
            then: Some(Node::Token(&token_b).boxed()),
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("abc", &token_a)],
            then: None,
        });

        let expected = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", &token_a),
                Branch {
                    regex: Regex::sequence("def"),
                    then: Some(parent.clone().boxed()),
                }
            ],
            then: Some(Node::Token(&token_b).boxed()),
        });

        parent.insert(child);

        assert!(parent == expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", parent, expected);
    }
}
