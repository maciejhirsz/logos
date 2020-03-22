use std::num::NonZeroUsize;
use std::ops::{Index, IndexMut};

// use crate::regex::Regex;

mod impls;
mod fork;
mod pattern;

pub use fork::{Fork, Branch};
pub use pattern::{Range, Pattern};

pub type Callback = syn::Ident;

#[cfg_attr(test, derive(Debug))]
pub struct Graph<Leaf> {
    nodes: Vec<Node<Leaf>>,
}

impl<Leaf> Graph<Leaf> {
    pub const fn new() -> Self {
        Graph {
            nodes: Vec::new()
        }
    }

    pub fn put<F, B>(&mut self, fun: F) -> NodeId
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

    pub fn nodes(&self) -> &[Node<Leaf>] {
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
pub struct Token<'a> {
    pub ident: &'a syn::Ident,
    pub callback: Option<Callback>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pat;

    #[test]
    fn create_a_loop() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("IDENT"));
        let root = graph.put(|id| {
            Fork::new(token).arm(pat!['a'..='z'], id)
        });

        assert_eq!(graph[token].body, NodeBody::Leaf("IDENT"));
        assert_eq!(
            graph[root].body,
            NodeBody::Fork(
                Fork::new(token).arm(pat!['a'..='z'], root)
            )
        );
    }
}
