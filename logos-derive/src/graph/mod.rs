use std::ops::Index;
use std::collections::BTreeMap as Map;

mod impls;
mod fork;
mod rope;
mod range;

pub use fork::Fork;
pub use rope::Rope;
pub use range::Range;

#[cfg_attr(test, derive(Debug))]
pub struct Graph<Leaf> {
    /// Internal storage of all allocated nodes. Once a node is
    /// put here, it should never be mutated.
    nodes: Vec<Node<Leaf>>,
    /// When merging two nodes into a new node, we store the two
    /// entry keys and the result, so that we don't merge the same
    /// two nodes multiple times.
    merges: Map<[NodeId; 2], NodeId>,
}

impl<Leaf> Graph<Leaf> {
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            merges: Map::default(),
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

    pub fn push<B>(&mut self, node: B) -> NodeId
    where
        B: Into<NodeBody<Leaf>>,
    {
        self.put(|_| node)
    }

    pub fn merge(&mut self, a: NodeId, b: NodeId) -> NodeId {
        if a == b {
            return a;
        }

        let sorted = if a > b { [b, a] } else { [a, b] };

        if let Some(merged) = self.merges.get(&sorted) {
            return *merged;
        }

        let [a, b] = sorted;

        if let (NodeBody::Rope(a), NodeBody::Rope(b)) = (&self[a].body, &self[b].body) {
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
        if let Some(miss) = miss {
            unimplemented!();
        }

        id
    }

    pub fn fork_off(&mut self, id: NodeId) -> Fork {
        match &self[id].body {
            NodeBody::Fork(fork) => fork.clone(),
            NodeBody::Rope(rope) => rope.clone().fork_off(self),
            NodeBody::Leaf(_) => Fork::new().miss(self[id].id),
        }
    }

    pub fn nodes(&self) -> &[Node<Leaf>] {
        &self.nodes
    }

    pub fn merges(&self) -> &Map<[NodeId; 2], NodeId> {
        &self.merges
    }

    fn merged(&mut self, key: [NodeId; 2], result: NodeId) -> NodeId {
        self.merges.insert(key, result);
        result
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
    /// Rope node, can lead to one state on match, one state on miss
    Rope(Rope),
    /// Leaf node, terminal state
    Leaf(Leaf),
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

        let token = graph.push(NodeBody::Leaf("IDENT"));
        let root = graph.put(|id| {
            Fork::new().branch('a'..='z', id).miss(token)
        });

        assert_eq!(graph[token].body, NodeBody::Leaf("IDENT"));
        assert_eq!(
            graph[root].body,
            NodeBody::Fork(
                Fork::new().branch('a'..='z', root).miss(token)
            )
        );
    }

    #[test]
    fn fork_off() {
        let mut graph = Graph::new();

        let leaf = graph.push(NodeBody::Leaf("LEAF"));
        let rope = graph.push(Rope::new("rope", leaf));
        let fork = graph.push(Fork::new().branch(b'!', leaf));

        assert_eq!(graph.fork_off(leaf), Fork::new().miss(leaf));
        assert_eq!(graph.fork_off(rope), Fork::new().branch(b'r', graph.nodes.len() - 1));
        assert_eq!(graph.fork_off(fork), Fork::new().branch(b'!', leaf));
    }
}
