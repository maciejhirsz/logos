use std::num::NonZeroUsize;
use std::ops::{Index, IndexMut};

// use crate::regex::Regex;

mod impls;

pub type Token<'a> = &'a syn::Ident;
pub type Callback = syn::Ident;
pub type Pattern = Vec<Range>;

#[cfg_attr(test, derive(PartialEq))]
pub struct Range(pub u8, pub u8);

impl From<u8> for Range {
    fn from(byte: u8) -> Range {
        Range(byte, byte)
    }
}

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
            nodes: Vec::new(),
        }
    }

    fn put<F, B>(&mut self, fun: F) -> NodeId
    where
        F: FnOnce(NodeId) -> B,
        B: Into<NodeBody<Leaf>>,
    {
        let id = self.nodes.len();

        self.nodes.push(Node {
            id,
            body: fun(id).into(),
        });

        id
    }

    fn nodes(&self) -> &[Node<Leaf>] {
        &self.nodes
    }
}

impl<Leaf> Index<NodeId> for Graph<Leaf> {
    type Output = Node<Leaf>;

    fn index(&self, id: NodeId) -> &Node<Leaf> {
        &self.nodes[id]
    }
}

pub type NodeId = usize;

#[cfg_attr(test, derive(PartialEq))]
pub struct Node<Leaf> {
    /// Id of this node in the graph
    pub id: NodeId,
    /// body of the node
    pub body: NodeBody<Leaf>,
}

#[cfg_attr(test, derive(PartialEq))]
pub enum NodeBody<Leaf> {
    /// Fork node, can lead to more than one state
    Fork(Fork),
    /// Leaf node, terminal state
    Leaf(Leaf),
}

#[cfg_attr(test, derive(PartialEq))]
pub struct Fork {
    /// Arms of the fork
    pub arms: Vec<Branch>,
    /// State to go to if no arms are matching
    pub miss: Option<NodeId>,
}

#[cfg_attr(test, derive(PartialEq))]
pub struct Branch {
    pub pattern: Pattern,
    pub then: NodeId,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Leaf<'a> {
    Token {
        token: Token<'a>,
        callback: Option<Callback>,
    },
    Trivia,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regex::Pattern;

    macro_rules! pat {
        ($($r:expr),*) => {vec![$($r.into()),*]};
    }

    #[test]
    fn create_a_loop() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("IDENT"));
        let root = graph.put(|id| Fork {
            arms: vec![
                Branch {
                    pattern: pat!['a'..='z'],
                    then: id,
                }
            ],
            miss: Some(token),
        });

        assert_eq!(graph[token].body, NodeBody::Leaf("IDENT"));
        assert_eq!(graph[root].body, NodeBody::Fork(Fork {
            arms: vec![
                Branch {
                    pattern: pat!['a'..='z'],
                    then: root,
                },
            ],
            miss: Some(token),
        }));
    }

    impl From<std::ops::RangeInclusive<u8>> for Range {
        fn from(range: std::ops::RangeInclusive<u8>) -> Range {
            Range(*range.start(), *range.end())
        }
    }

    impl From<std::ops::RangeInclusive<char>> for Range {
        fn from(range: std::ops::RangeInclusive<char>) -> Range {
            Range(*range.start() as u8, *range.end() as u8)
        }
    }
}
