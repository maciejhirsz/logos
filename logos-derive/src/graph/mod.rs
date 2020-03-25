use std::ops::Index;
use std::collections::BTreeMap as Map;
use std::collections::btree_map::Entry;
use std::hash::{Hash, Hasher};

use fnv::FnvHasher;

mod impls;
mod fork;
mod rope;
mod range;
mod regex;

pub use fork::Fork;
pub use rope::Rope;
pub use range::Range;

pub struct Graph<Leaf> {
    /// Internal storage of all allocated nodes. Once a node is
    /// put here, it should never be mutated.
    nodes: Vec<Option<Node<Leaf>>>,
    /// When merging two nodes into a new node, we store the two
    /// entry keys and the result, so that we don't merge the same
    /// two nodes multiple times.
    ///
    /// Most of the time the entry we want to find will be the last
    /// one that has been inserted, so we can use a vec with reverse
    /// order search to get O(1) searches much faster than any *Map.
    merges: Vec<([NodeId; 2], NodeId)>,
    /// Another map used for accounting. Before `.push`ing a new node
    /// onto the graph (inserts are exempt), we hash it and find if
    /// an identical(!) node has been created before.
    hashes: Map<u64, NodeId>,
}

/// Unique reserved NodeId. This mustn't implement Clone.
pub struct ReservedId(NodeId);

impl ReservedId {
    pub fn get(&self) -> NodeId {
        self.0
    }
}

impl<Leaf> Graph<Leaf> {
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            merges: Vec::new(),
            hashes: Map::new(),
        }
    }

    pub fn reserve(&mut self) -> ReservedId {
        let id = self.nodes.len();

        self.nodes.push(None);

        ReservedId(id)
    }

    pub fn insert<N>(&mut self, id: ReservedId, node: N) -> NodeId
    where
        N: Into<Node<Leaf>>,
    {
        self.nodes[id.0] = Some(node.into());

        id.0
    }

    pub fn push<B>(&mut self, node: B) -> NodeId
    where
        B: Into<Node<Leaf>>,
    {
        let node = node.into();

        if let Node::Leaf(_) = node {
            return self.push_unchecked(node);
        }

        let mut hasher = FnvHasher::default();
        node.hash(&mut hasher);

        match self.hashes.entry(hasher.finish()) {
            Entry::Occupied(occupied) => {
                let id = *occupied.get();

                if self[id].eq(&node) {
                    return id;
                }
            },
            Entry::Vacant(vacant) => {
                vacant.insert(self.nodes.len());
            },
        }

        self.push_unchecked(node)
    }

    fn push_unchecked(&mut self, node: Node<Leaf>) -> NodeId{
        let id = self.nodes.len();
        self.nodes.push(Some(node));
        id
    }

    pub fn merge(&mut self, a: NodeId, b: NodeId) -> NodeId {
        if a == b {
            return a;
        }

        let sorted = if a > b { [b, a] } else { [a, b] };

        if let Some((_, merged)) = self.merges.iter().rev().find(|(key, _)| *key == sorted) {
            return *merged;
        }

        let [a, b] = sorted;

        if let (Node::Rope(a), Node::Rope(b)) = (&self[a], &self[b]) {
            if let Some(prefix) = a.prefix(b) {
                let (a, b) = (a.clone(), b.clone());

                let a = a.remainder(prefix.len(), self);
                let b = b.remainder(prefix.len(), self);

                let then = self.merge(a, b);
                let merged = self.push(Rope::new(prefix, then));

                return self.merged(sorted, merged);
            }
        }

        let mut fork = self.fork_off(a);

        fork.merge(self.fork_off(b), self);

        let merged = self.push(fork);
        self.merged(sorted, merged)
    }

    pub fn push_miss(&mut self, id: NodeId, miss: Option<NodeId>) -> NodeId {
        if let Some(_) = miss {
            unimplemented!();
        }

        id
    }

    pub fn fork_off(&mut self, id: NodeId) -> Fork {
        match &self[id] {
            Node::Fork(fork) => fork.clone(),
            Node::Rope(rope) => rope.clone().fork_off(self),
            Node::Leaf(_) => Fork::new().miss(id),
        }
    }

    pub fn nodes(&self) -> &[Option<Node<Leaf>>] {
        &self.nodes
    }

    pub fn merges(&self) -> &[([NodeId; 2], NodeId)] {
        &self.merges
    }

    /// Removes all nodes that have no references
    pub fn shake(&mut self) {
        let root = match self.nodes.len().checked_sub(1) {
            Some(id) => id,
            None => return,
        };

        let mut filter = vec![false; self.nodes.len()];

        filter[root] = true;

        self[root].shake(self, &mut filter);

        for (id, referenced) in filter.into_iter().enumerate() {
            if !referenced {
                self.nodes[id] = None;
            }
        }
    }

    fn merged(&mut self, key: [NodeId; 2], result: NodeId) -> NodeId {
        self.merges.push((key, result));
        result
    }
}

impl<Leaf> Index<NodeId> for Graph<Leaf> {
    type Output = Node<Leaf>;

    fn index(&self, id: NodeId) -> &Node<Leaf> {
        self.nodes[id].as_ref().expect("Indexing into a reserved node")
    }
}

pub type NodeId = usize;

#[cfg_attr(test, derive(PartialEq))]
pub enum Node<Leaf> {
    /// Fork node, can lead to more than one state
    Fork(Fork),
    /// Rope node, can lead to one state on match, one state on miss
    Rope(Rope),
    /// Leaf node, terminal state
    Leaf(Leaf),
}

impl<Leaf> Node<Leaf> {
    fn eq(&self, other: &Node<Leaf>) -> bool {
        match (self, other) {
            (Node::Fork(a), Node::Fork(b)) => a == b,
            (Node::Rope(a), Node::Rope(b)) => a == b,
            _ => false,
        }
    }

    fn shake(&self, graph: &Graph<Leaf>, filter: &mut [bool]) {
        match self {
            Node::Fork(fork) => fork.shake(graph, filter),
            Node::Rope(rope) => rope.shake(graph, filter),
            Node::Leaf(_) => (),
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
pub struct Token {
    pub ident: syn::Ident,
    pub callback: Option<syn::Ident>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn create_a_loop() {
        let mut graph = Graph::new();

        let token = graph.push(Node::Leaf("IDENT"));
        let id = graph.reserve();
        let fork = Fork::new().branch('a'..='z', id.get()).miss(token);
        let root = graph.insert(id, fork);

        assert_eq!(graph[token], Node::Leaf("IDENT"));
        assert_eq!(
            graph[root],
            Fork::new().branch('a'..='z', root).miss(token),
        );
    }

    #[test]
    fn fork_off() {
        let mut graph = Graph::new();

        let leaf = graph.push(Node::Leaf("LEAF"));
        let rope = graph.push(Rope::new("rope", leaf));
        let fork = graph.push(Fork::new().branch(b'!', leaf));

        assert_eq!(graph.fork_off(leaf), Fork::new().miss(leaf));
        assert_eq!(graph.fork_off(rope), Fork::new().branch(b'r', graph.nodes.len() - 1));
        assert_eq!(graph.fork_off(fork), Fork::new().branch(b'!', leaf));
    }
}
