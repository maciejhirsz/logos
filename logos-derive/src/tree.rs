use syn::Ident;
use std::mem;
use std::cmp::Ordering;
use regex::{Regex, RepetitionFlag};
use util::OptionExt;

#[derive(Debug, Clone)]
pub struct Branch<'a> {
    pub regex: Regex,
    pub flag: Option<RepetitionFlag>,
    pub then: Box<Node<'a>>,
}

#[derive(Debug, Clone, Default)]
pub struct Fork<'a> {
    pub arms: Vec<Branch<'a>>,
    pub default: Option<&'a Ident>,
}

#[derive(Debug, Clone)]
pub enum Node<'a> {
    Branch(Branch<'a>),
    Fork(Fork<'a>),
    Leaf(&'a Ident),
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
            flag: None,
            then: Node::Leaf(token).boxed(),
        }
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
                            flag: None,
                            then: Node::from(branch).boxed(),
                        });

                        // Append old branch to the newly created fork
                        other.then.insert(old);

                        return;
                    }
                }

                // Sort arms of the fork, simple bytes in alphabetical order first, patterns last
                match self.arms.binary_search_by(|other| {
                    other.regex.first().partial_cmp(branch.regex.first()).unwrap_or_else(|| Ordering::Greater)
                }) {
                    Ok(index) => {
                        self.arms[index].then.insert(branch);
                    },
                    Err(index) => {
                        self.arms.insert(index, branch.into());
                    },
                }
            },
            Node::Leaf(leaf) => {
                self.default.insert(leaf, |_| panic!("Two token variants cannot be produced by the same explicit path!"));
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
                flag: None,
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
        mem::replace(self, node.into());
    }

    pub fn set_flag(&mut self, flag: RepetitionFlag) {
        match self {
            Node::Branch(branch) => branch.flag.insert(flag, |_| panic!("Flag was already set!")),
            Node::Fork(fork) => {
                for branch in fork.arms.iter_mut() {
                    branch.flag.insert(flag, |_| panic!("Flag was already set!"))
                }
            },
            Node::Leaf(_) => {},
        }
    }

    // Tests whether there is a branch on the node that doesn't consume any more bytes
    pub fn can_be_empty(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            Node::Branch(branch) => branch.flag == Some(RepetitionFlag::ZeroOrMore),
            Node::Fork(fork) => {
                fork.arms
                    .iter()
                    .any(|branch| branch.flag == Some(RepetitionFlag::ZeroOrMore))
            }
        }
    }

    /// Tests whether the branch produces a token on all leaves without any tests.
    pub fn exhaustive(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            Node::Branch(branch) => {
                branch.regex.len() == 1 && branch.then.exhaustive()
            },
            Node::Fork(fork) => {
                fork.default.is_some()
                    && fork.arms.iter().all(|branch| {
                        branch.regex.is_byte() && branch.then.exhaustive()
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
                        .filter(|(_, branch)| branch.regex.first().is_repeat())
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

    /// Checks whether the tree can produce only a single token, if so return that token
    pub fn only_leaf(&self) -> Option<&'a Ident> {
        match self {
            Node::Leaf(leaf) => Some(leaf),
            Node::Branch(branch) => branch.then.only_leaf(),
            Node::Fork(_) => None,
        }
    }

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}
