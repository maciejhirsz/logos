use std::num::NonZeroUsize;
use std::ops::{Range, Index, IndexMut};

// use crate::regex::Regex;

mod impls;

pub type Token<'a> = &'a syn::Ident;
pub type Callback = syn::Ident;
pub type Pattern = Vec<Range<u8>>;

#[cfg_attr(test, derive(Debug))]
pub struct Graph<Leaf> {
    nodes: Vec<Node<Leaf>>,
}

impl<Leaf> Graph<Leaf> {
    fn new() -> Self
    where
        Leaf: Default,
    {
        Graph {
            // Start with one dummy entry, so that NodeId doesn't
            // start with 0!
            nodes: vec![NodeBody::Leaf(Leaf::default()).into()],
        }
    }

    fn put<F, B>(&mut self, fun: F) -> NodeId
    where
        F: FnOnce(NodeId) -> B,
        B: Into<NodeBody<Leaf>>,
    {
        let id = NodeId::new(self.nodes.len()).expect("0 sized graph");

        self.nodes.push(Node::new(fun(id).into()));

        id
    }

    fn nodes(&self) -> &[Node<Leaf>] {
        &self.nodes[1..]
    }
}

impl<Leaf> Index<NodeId> for Graph<Leaf> {
    type Output = Node<Leaf>;

    fn index(&self, id: NodeId) -> &Node<Leaf> {
        &self.nodes[id.get()]
    }
}

impl<Leaf> IndexMut<NodeId> for Graph<Leaf> {
    fn index_mut(&mut self, id: NodeId) -> &mut Node<Leaf> {
        &mut self.nodes[id.get()]
    }
}

pub type NodeId = NonZeroUsize;

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Node<Leaf> {
    /// Reference count to this node
    rc: usize,
    /// body of the node
    pub body: NodeBody<Leaf>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum NodeBody<Leaf> {
    /// Fork node, can lead to more than one state
    Fork(Fork),
    /// Leaf node, terminal state
    Leaf(Leaf),
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Fork {
    /// Arms of the fork
    pub arms: Vec<Branch>,
    /// State to go to if no arms are matching
    pub miss: Option<NodeId>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Branch {
    pub pattern: Pattern,
    pub then: Option<NodeId>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Leaf<'a> {
    Token {
        token: Token<'a>,
        callback: Option<Callback>,
    },
    Trivia,
}

impl<Leaf> Node<Leaf> {
    pub fn new<T>(body: T) -> Self
    where
        T: Into<NodeBody<Leaf>>,
    {
        Node {
            rc: 0,
            body: body.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::Pattern;

    #[test]
    fn create_a_loop() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("IDENT"));
        let root = graph.put(|id| Fork {
            arms: vec![
                Branch {
                    pattern: vec![b'a'..b'z'],
                    then: Some(id),
                }
            ],
            miss: Some(token),
        });

        assert_eq!(graph[token].body, NodeBody::Leaf("IDENT"));
        assert_eq!(graph[root].body, NodeBody::Fork(Fork {
            arms: vec![
                Branch {
                    pattern: vec![b'a'..b'z'],
                    then: Some(root),
                },
            ],
            miss: Some(token),
        }));
    }
}
// impl<'a> From<Token<'a>> for Leaf<'a> {
//     fn from(token: Token<'a>) -> Self {
//         Leaf::Token {
//             token,
//             callback: None,
//         }
//     }
// }