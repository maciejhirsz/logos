use std::{mem, fmt};
use std::cmp::Ordering;
use regex::{Regex, RepetitionFlag};

pub type Token<'a> = &'a ::syn::Ident;
pub type Callback = ::syn::Ident;

#[derive(Clone, Default, PartialEq)]
pub struct Branch<'a> {
    pub regex: Regex,
    pub then: Option<Box<Node<'a>>>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Leaf<'a> {
    pub token: Token<'a>,
    pub callback: Option<Callback>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    Leaf(Leaf<'a>),
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

impl<'a> fmt::Debug for Leaf<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.token)?;

        if let Some(ref callback) = self.callback {
            write!(f, " ({})", callback)?;
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

impl<'a> From<Token<'a>> for Leaf<'a> {
    fn from(token: Token<'a>) -> Self {
        Leaf {
            token,
            callback: None,
        }
    }
}

impl<'a> From<Token<'a>> for Node<'a> {
    fn from(token: Token<'a>) -> Self {
        Node::Leaf(token.into())
    }
}

impl<'a> From<Leaf<'a>> for Node<'a> {
    fn from(leaf: Leaf<'a>) -> Self {
        Node::Leaf(leaf)
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
        if fork.arms.len() == 0 {
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

impl<'a> Leaf<'a> {
    fn take(&mut self) -> Leaf<'a> {
        Leaf {
            token: self.token,
            callback: self.callback.take(),
        }
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
            Node::Branch(branch) => {
                // If possible, we unwind repeat forks and collapse maybe forks.
                self.unwind();
                self.collapse();

                self.insert_branch(branch);
            },
            Node::Leaf(leaf) => {
                if self.then.is_none() {
                    assert!(
                        self.kind == ForkKind::Plain,
                        "Internal Error: Invalid fork construction: {:#?}", self
                    );

                    self.kind = ForkKind::Maybe;
                    self.then = Some(Node::Leaf(leaf).boxed());
                } else {
                    self.collapse();

                    assert!(
                        self.kind != ForkKind::Plain,
                        "Internal Error: Invalid fork construction: {:#?}", self
                    );

                    self.kind = ForkKind::Maybe;
                    self.then = Some(Node::Leaf(leaf).boxed());
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

        // FIXME!
        //
        // This is kind of a hack that prevents us from creating intersections for
        // identifiers all the way down, blowing up the stack!
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
                    then: None,
                });

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
                self.arms[index].insert_then(branch.to_node().map(Box::new));
            },
            Err(index) => {
                self.arms.insert(index, branch.into());
            },
        }
    }

    pub fn insert_then(&mut self, other: Option<Box<Node<'a>>>) {
        match self.then {
            Some(ref mut node) => {
                match other {
                    Some(other) => node.insert(*other),
                    None => node.make_maybe_fork(),
                }
            }
            ref mut then => {
                if other.is_some() {
                    assert!(
                        self.kind != ForkKind::Repeat,
                        "Internal Error: Invalid fork construction"
                    );

                    self.kind = ForkKind::Maybe;

                    *then = other;
                }
            },
        }
    }

    /// Unwinds a Repeat fork into a Maybe fork
    pub fn unwind(&mut self) {
        if self.kind != ForkKind::Repeat {
            return;
        }

        let repeat = Node::from(self.clone());

        for branch in self.arms.iter_mut() {
            branch.chain(&repeat);
        }

        let mut then = self.then.take();

        let move_back = if let Some(ref mut then) = then {
            match **then {
                Node::Fork(ref mut fork) if fork.kind == ForkKind::Plain => {
                    for branch in fork.arms.drain(..) {
                        self.insert_branch(branch);
                    }

                    false
                },
                Node::Branch(ref mut branch) => {
                    self.insert_branch(branch.clone());

                    false
                },
                _ => true,
            }
        } else {
            true
        };

        if move_back {
            self.then = then;
            self.kind = ForkKind::Maybe;
        } else {
            self.kind = ForkKind::Plain;
        }
    }

    // Attempts to collapse a Maybe fork into a Plain fork.
    // If `then` on this fork is a `Token`, or if it isn't
    // set, then it will remain a Maybe fork.
    pub fn collapse(&mut self) {
        if self.kind != ForkKind::Maybe {
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
            self.kind = ForkKind::Plain;
            self.insert(*then);
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
    pub fn new(regex: Regex, leaf: Leaf<'a>) -> Self {
        if regex.len() == 0 {
            Node::Leaf(leaf)
        } else {
            Node::Branch(Branch {
                regex,
                then: Some(Node::from(leaf).boxed()),
            })
        }
    }

    fn to_mut_fork(&mut self) -> &mut Fork<'a> {
        let fork = match self {
            Node::Fork(fork) => return fork,
            Node::Branch(ref mut branch) => {
                let branch = mem::replace(branch, Branch {
                    regex: Regex::default(),
                    then: None,
                });

                Fork {
                    kind: ForkKind::Plain,
                    arms: vec![branch],
                    then: None,
                }
            },
            Node::Leaf(leaf) => {
                Fork {
                    kind: ForkKind::Maybe,
                    arms: vec![],
                    then: Some(Node::Leaf(leaf.take()).boxed()),
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

    fn make_maybe_fork(&mut self) {
        let fork = match self {
            Node::Fork(fork) => {
                assert!(fork.kind != ForkKind::Repeat);

                return fork.kind = ForkKind::Maybe;
            },
            Node::Branch(ref mut branch) => {
                let branch = mem::replace(branch, Branch {
                    regex: Regex::default(),
                    then: None,
                });

                Fork {
                    kind: ForkKind::Maybe,
                    arms: vec![branch],
                    then: None,
                }
            },
            Node::Leaf(_) => return,
        };

        mem::replace(self, Node::Fork(fork));
    }

    pub fn insert(&mut self, then: Node<'a>) {
        self.to_mut_fork().insert(then);

        let then = match self {
            Node::Fork(fork) if fork.arms.len() == 0 => {
                fork.then.take()
            },
            _ => None
        };

        if let Some(then) = then {
            mem::replace(self, *then);
        }
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
    pub fn is_exhaustive(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            Node::Branch(_) => false,
            Node::Fork(fork) => {
                let exhaustive_nones = if fork.kind == ForkKind::Repeat { true } else { false };

                fork.then.as_ref().map(|then| then.is_exhaustive()).unwrap_or(false)
                    && (fork.kind != ForkKind::Repeat || fork.arms.len() == 1)
                    && fork.arms.iter().all(|branch| {
                        branch.regex.len() == 1
                            && branch.then.as_ref().map(|then| then.is_exhaustive()).unwrap_or(exhaustive_nones)
                    })
            },
        }
    }

    /// Tests whether all branches have a `then` node set to `Some`.
    pub fn is_bounded(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            Node::Branch(branch) => branch.then.is_some(),
            Node::Fork(fork) => fork.arms.iter().all(|branch| branch.then.is_some()),
        }
    }

    pub fn fallback(&mut self) -> Option<Branch<'a>> {
        match self {
            Node::Fork(fork) => {
                if fork.kind != ForkKind::Maybe {
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

    fn is_token(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
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

    fn branch<'a>(regex: &str, leaf: Leaf<'a>) -> Branch<'a> {
        Branch {
            regex: Regex::sequence(regex),
            then: Some(Node::Leaf(leaf).boxed()),
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
            then: Some(Node::Leaf(Leaf::from(&token)).boxed()),
        };

        assert_eq!(branch.to_node(), Some(Node::Leaf(Leaf::from(&token))));
    }

    #[test]
    fn insert_branch_into_branch() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let branch_a = branch("abc", Leaf::from(&token_a));
        let branch_b = branch("def", Leaf::from(&token_b));

        let mut parent = branch("abc", Leaf::from(&token_a)).to_node().unwrap();
        let child = branch("def", Leaf::from(&token_b)).to_node().unwrap();

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

        let mut parent = branch("abc", Leaf::from(&token_a)).to_node().unwrap();
        let child = Node::Leaf(Leaf::from(&token_b));

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("abc", Leaf::from(&token_a))],
            then: Some(Node::Leaf(Leaf::from(&token_b)).boxed()),
        }));
    }

    #[test]
    fn insert_a_branch_into_a_token() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Leaf(Leaf::from(&token_a));
        let child = branch("xyz", Leaf::from(&token_b)).to_node().unwrap();

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("xyz", Leaf::from(&token_b))],
            then: Some(Node::Leaf(Leaf::from(&token_a)).boxed()),
        }));
    }

    #[test]
    fn insert_a_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("abc", Leaf::from(&token_a))],
            then: None,
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("def", Leaf::from(&token_b))],
            then: None,
        });

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![
                branch("abc", Leaf::from(&token_a)),
                branch("def", Leaf::from(&token_b)),
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
            arms: vec![branch("abc", Leaf::from(&token_a))],
            then: None,
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("def", Leaf::from(&token_b))],
            then: Some(Node::Leaf(Leaf::from(&token_x)).boxed()),
        });

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", Leaf::from(&token_a)),
                branch("def", Leaf::from(&token_b)),
            ],
            then: Some(Node::Leaf(Leaf::from(&token_x)).boxed()),
        }));
    }

    #[test]
    fn insert_a_fork_into_a_maybe_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");
        let token_x = token("XYZ");

        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("abc", Leaf::from(&token_a))],
            then: Some(Node::Leaf(Leaf::from(&token_x)).boxed()),
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("def", Leaf::from(&token_b))],
            then: None,
        });

        parent.insert(child);

        assert_eq!(parent, Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", Leaf::from(&token_a)),
                branch("def", Leaf::from(&token_b)),
            ],
            then: Some(Node::Leaf(Leaf::from(&token_x)).boxed()),
        }));
    }

    #[test]
    fn collapsing_a_fork() {
        let token_a = token("ABC");

        let mut fork = Fork {
            kind: ForkKind::Maybe,
            arms: vec![Branch::new(Regex::sequence("abc"))],
            then: Some(Node::Leaf(Leaf::from(&token_a)).boxed()),
        };

        let expected = Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("abc", Leaf::from(&token_a))],
            then: Some(Node::Leaf(Leaf::from(&token_a)).boxed()),
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
            then: Some(Node::Leaf(Leaf::from(&token_a)).boxed()),
        };

        let expected = Fork {
            kind: ForkKind::Maybe,
            arms: vec![Branch {
                regex: Regex::sequence("abc"),
                then: Some(Node::Fork(fork.clone()).boxed()),
            }],
            then: Some(Node::Leaf(Leaf::from(&token_a)).boxed()),
        };

        fork.unwind();

        assert!(fork == expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", fork, expected);
    }

    #[test]
    fn insert_a_repeat_fork_into_a_fork() {
        let token_a = token("ABC");
        let token_b = token("DEF");

        let mut parent = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("abc", Leaf::from(&token_a))],
            then: None,
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Repeat,
            arms: vec![Branch::new(Regex::sequence("def"))],
            then: Some(Node::Leaf(Leaf::from(&token_b)).boxed()),
        });

        let expected = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", Leaf::from(&token_a)),
                Branch {
                    regex: Regex::sequence("def"),
                    then: Some(child.clone().boxed()),
                }
            ],
            then: Some(Node::Leaf(Leaf::from(&token_b)).boxed()),
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
            then: Some(Node::Leaf(Leaf::from(&token_b)).boxed()),
        });

        let child = Node::Fork(Fork {
            kind: ForkKind::Plain,
            arms: vec![branch("abc", Leaf::from(&token_a))],
            then: None,
        });

        let expected = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                branch("abc", Leaf::from(&token_a)),
                Branch {
                    regex: Regex::sequence("def"),
                    then: Some(parent.clone().boxed()),
                }
            ],
            then: Some(Node::Leaf(Leaf::from(&token_b)).boxed()),
        });

        parent.insert(child);

        assert!(parent == expected, "Not equal:\n\nGOT {:#?}\n\nEXPECTED {:#?}", parent, expected);
    }

    #[test]
    fn tree_is_exhaustive() {
        // '=' -> MAYBE [
        //     '=' -> MAYBE [
        //         '=' -> TOKEN "OpStrictEquality"
        //     ] -> TOKEN "OpEquality",
        //     '>' -> TOKEN "FatArrow"
        // ] -> TOKEN "OpAssign",
        //
        let arrow = token("FatArrow");
        let assign = token("OpAssign");
        let eq = token("OpEquality");
        let seq = token("OpStrictEquality");
        let arrow = Leaf::from(&arrow);
        let assign = Leaf::from(&assign);
        let eq = Leaf::from(&eq);
        let seq = Leaf::from(&seq);

        let seq_or_eq = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![branch("=", seq)],
            then: Some(Node::Leaf(eq).boxed()),
        });

        let tree = Node::Fork(Fork {
            kind: ForkKind::Maybe,
            arms: vec![
                Branch {
                    regex: Regex::sequence("="),
                    then: Some(Box::new(seq_or_eq)),
                },
                branch(">", arrow),
            ],
            then: Some(Node::Leaf(assign).boxed()),
        });

        assert!(tree.is_exhaustive(), "Tree is not is_exhaustive! {:#?}", tree);
    }
}
