use syn::Ident;
use std::{mem, fmt};
use std::cmp::Ordering;
use regex::{Regex, RepetitionFlag};
use util::OptionExt;

#[derive(Debug, Clone)]
pub struct Branch<'a> {
    pub regex: Regex,
    pub then: Box<Node<'a>>,
}

#[derive(Debug, Clone, Default)]
pub struct Fork<'a> {
    pub arms: Vec<Branch<'a>>,
    pub default: Option<&'a Ident>,
}

#[derive(Clone)]
pub enum Node<'a> {
    Branch(Branch<'a>),
    Fork(Fork<'a>),
    Leaf(&'a Ident),
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Branch(branch) => branch.fmt(f),
            Node::Fork(fork) => fork.fmt(f),
            Node::Leaf(leaf) => write!(f, "Leaf(\"{}\")", leaf),
        }
    }
}

impl<'a> From<&'a Ident> for Node<'a> {
    fn from(token: &'a Ident) -> Self {
        Node::Leaf(token)
    }
}

impl<'a> From<Branch<'a>> for Node<'a> {
    fn from(branch: Branch<'a>) -> Self {
        if branch.regex.len() == 0 {
            *branch.then
        } else {
            Node::Branch(branch)
        }
    }
}

impl<'a> Branch<'a> {
    pub fn new(regex: Regex, token: &'a Ident) -> Self {
        Branch {
            regex,
            then: Node::Leaf(token).boxed(),
        }
    }

    pub fn compare(&self, other: &Branch<'a>) -> Ordering {
        other.regex.first().partial_cmp(self.regex.first()).unwrap_or_else(|| Ordering::Greater)
    }
}

impl<'a> From<Fork<'a>> for Node<'a> {
    fn from(fork: Fork<'a>) -> Self {
        Node::Fork(fork)
    }
}

impl<'a> Fork<'a> {
    pub fn insert<N>(&mut self, then: N)
    where
        N: Into<Node<'a>>
    {
        match then.into() {
            Node::Branch(mut branch) => {
                if branch.regex.len() == 0 {
                    return self.insert(*branch.then);
                }

                // Look for a branch that matches the same prefix
                for other in self.arms.iter_mut() {
                    // We got a match!
                    if let Some(regex) = branch.regex.match_split(&mut other.regex) {
                        // Create a new branch with the common prefix in place of the old one,
                        let old = mem::replace(other, Branch {
                            regex,
                            then: Node::from(branch).boxed(),
                        });

                        // Append old branch to the newly created fork
                        other.then.insert(old);

                        return;
                    }

                    if let Some(prefix) = branch.regex.common_prefix(&other.regex) {
                        let regex = Regex::from(prefix);

                        let mut fork = Fork::default();
                        let mut new = Branch {
                            regex,
                            then: Node::from(fork).boxed()
                        };

                        if branch.regex.first() == new.regex.first() {
                            let mut other = other.clone();
                            let mut old = mem::replace(&mut branch, new);

                            old.regex.unshift();
                            other.regex.unshift();

                            branch.then.insert(old);
                            branch.then.insert(other);

                            // panic!("BRANCH! {:#?}", branch);


                            // return;

                        } else if other.regex.first() == new.regex.first() {
                            let mut branch = branch.clone();
                            let mut old = mem::replace(other, new);

                            old.regex.unshift();
                            branch.regex.unshift();

                            other.then.insert(old);
                            // other.then.insert(branch);


                            // panic!("OTHER! {:#?}\nBRANCH {:#?}", other, branch);

                            // return;
                            // let mut old = mem::replace(other, new);

                            // old.regex.unshift();

                            // other.then.insert(old);

                            // return;
                        }
                    }
                }

                // Sort arms of the fork, simple bytes in alphabetical order first, patterns last
                match self.arms.binary_search_by(|other| branch.compare(other)) {
                    Ok(index) => {
                        println!("Found matching index for {:#?}\n\nat {:#?}\n\n--------", branch, self.arms[index]);
                        println!("{:?}", branch.compare(&self.arms[index]));

                        self.arms[index].then.insert(branch);
                    },
                    Err(index) => {
                        self.arms.insert(index, branch.into());
                    },
                }
            },
            Node::Leaf(leaf) => {
                self.default.insert(leaf, |old| {
                    if old != &leaf {
                        panic!("Two token variants cannot be produced by the same explicit path: {} and {}", leaf, old);
                    }
                });
            },
            Node::Fork(other) => {
                if let Some(leaf) = other.default {
                    self.insert(leaf);
                }

                for branch in other.arms {
                    self.insert(branch);
                }
            }
        }
    }
}

impl<'a> Node<'a> {
    pub fn new(regex: Regex, token: &'a Ident) -> Self {
        if regex.len() == 0 {
            Node::Leaf(token)
        } else {
            Node::Branch(Branch {
                regex,
                then: Node::Leaf(token).boxed()
            })
        }
    }

    pub fn insert<N>(&mut self, then: N)
    where
        N: Into<Node<'a>>
    {
        match self {
            Node::Fork(_) => {},
            _ => {
                let replaced = mem::replace(self, Node::Fork(Fork::default()));

                match replaced {
                    Node::Branch(branch) => self.insert(branch),

                    // FIXME: set default without resorting to creating a new Regex here
                    Node::Leaf(token)    => self.insert(token),

                    _ => unreachable!(),
                }
            }
        }

        if let Node::Fork(fork) = self {
            fork.insert(then);
        }
    }

    pub fn replace<N>(&mut self, node: N)
    where
        N: Into<Node<'a>>
    {
        match self {
            Node::Leaf(_) => {},
            _ => panic!("Throwing non-leaf node away! {:#?}", self)
        }

        mem::replace(self, node.into());
    }

    pub fn set_repeat(&mut self, flag: RepetitionFlag) {
        match self {
            Node::Branch(branch) => branch.regex.repeat = flag,
            Node::Fork(fork) => {
                for branch in fork.arms.iter_mut() {
                    branch.regex.repeat = flag;
                }
            },
            Node::Leaf(_) => {},
        }
    }

    pub fn chain(&mut self, then: Node<'a>) {
        match self {
            Node::Branch(branch) => branch.then.replace(then),
            Node::Fork(fork) => {
                for branch in fork.arms.iter_mut() {
                    branch.then.replace(then.clone());
                }
            },
            Node::Leaf(_) => {
                mem::replace(self, then);
            }
        }
    }

    /// Tests whether the branch produces a token on all leaves without any tests.
    pub fn exhaustive(&self) -> bool {
        use self::RepetitionFlag::*;

        match self {
            Node::Leaf(_) => true,
            Node::Branch(branch) => {
                branch.regex.len() == 1
                    && (branch.regex.repeat == ZeroOrMore || branch.regex.repeat == ZeroOrOne)
                    && branch.then.exhaustive()
            },
            Node::Fork(fork) => {
                fork.default.is_some()
                    && fork.arms.iter().all(|branch| {
                        branch.regex.len() == 1
                            && (branch.regex.repeat == ZeroOrMore || branch.regex.repeat == ZeroOrOne)
                            && branch.then.exhaustive()
                    })
            }
        }
    }

    pub fn fallback(&mut self) -> Option<Branch<'a>> {
        match self {
            Node::Fork(fork) => {
                // This is a bit weird, but it basically checks if the fork
                // has one and only one branch that start with a repeat regex,
                // and if so, it removes that branch and returns it.
                //
                // FIXME: This should check if all other branches in the tree
                //        are specializations of that regex
                let idxs =
                    fork.arms
                        .iter()
                        .enumerate()
                        .filter(|(_, branch)| {
                            branch.regex.repeat != RepetitionFlag::One && branch.regex.first().weight() > 1
                        })
                        .map(|(idx, _)| idx)
                        .collect::<Vec<_>>();

                if idxs.len() == 1 {
                    Some(fork.arms.remove(idxs[0]))
                } else {
                    None
                }
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
            Node::Leaf(token) => insert(vec, token),
            Node::Branch(branch) => branch.then.get_tokens(vec),
            Node::Fork(fork) => {
                for branch in fork.arms.iter() {
                    branch.then.get_tokens(vec);
                }
                if let Some(token) = fork.default {
                    insert(vec, token)
                }
            }
        }
    }

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}
