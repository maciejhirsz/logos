use std::cmp::Ordering;
use std::num::NonZeroU32;
use std::collections::BTreeMap as Map;
use std::collections::btree_map::Entry;
use std::ops::{Index, IndexMut};
use std::hash::{Hash, Hasher};

use fnv::FnvHasher;

mod impls;
mod meta;
mod fork;
mod rope;
mod range;
mod regex;

pub use self::meta::Meta;
pub use self::fork::Fork;
pub use self::rope::Rope;
pub use self::range::Range;

/// Disambiguation error during the attempt to merge two leaf
/// nodes with the same priority
#[derive(Debug)]
pub struct DisambiguationError(pub NodeId, pub NodeId);

pub type Result<T> = std::result::Result<T, DisambiguationError>;

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

pub trait Disambiguate {
    fn cmp(left: &Self, right: &Self) -> Ordering;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(NonZeroU32);

impl NodeId {
    fn get(self) -> usize {
        self.0.get() as usize
    }

    fn new(n: usize) -> NodeId {
        NodeId(NonZeroU32::new(n as u32).expect("Invalid NodeId"))
    }
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
            // Start with an empty slot so we can start
            // counting NodeIds from 1 and use NonZero
            // optimizations
            nodes: vec![None],
            merges: Vec::new(),
            hashes: Map::new(),
        }
    }

    fn next_id(&self) -> NodeId {
        NodeId::new(self.nodes.len())
    }

    /// Reserve an empty slot for a node on the graph and return an
    /// id for it. `ReservedId` cannot be cloned, and must be consumed
    /// by calling `insert` on the graph.
    pub fn reserve(&mut self) -> ReservedId {
        let id = self.next_id();

        self.nodes.push(None);

        ReservedId(id)
    }

    /// Insert a node at a given, previously reserved id. Returns the
    /// inserted `NodeId`.
    pub fn insert<N>(&mut self, id: ReservedId, node: N) -> NodeId
    where
        N: Into<Node<Leaf>>,
    {
        self.nodes[id.0.get()] = Some(node.into());

        id.0
    }

    /// Push a node onto the graph and get an id to it. If an identical
    /// node has already been pushed on the graph, it will return the id
    /// of that node instead.
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

        let next_id = self.next_id();

        match self.hashes.entry(hasher.finish()) {
            Entry::Occupied(occupied) => {
                let id = *occupied.get();

                if self[id].eq(&node) {
                    return id;
                }
            },
            Entry::Vacant(vacant) => {
                vacant.insert(next_id);
            },
        }

        self.push_unchecked(node)
    }

    fn push_unchecked(&mut self, node: Node<Leaf>) -> NodeId {
        let id = self.next_id();

        self.nodes.push(Some(node));

        id
    }

    /// Merge the nodes at id `a` and `b`, returning a new id.
    pub fn merge(&mut self, a: NodeId, b: NodeId) -> Result<NodeId>
    where
        Leaf: Disambiguate,
    {
        if a == b {
            return Ok(a);
        }

        match (self.get(a), self.get(b)) {
            (None, None) => {
                panic!("Merging two reserved nodes!");
            },
            // Merging a leaf with an empty slot would produce
            // an empty self-referencing fork.
            (Some(Node::Leaf(_)), None) => return Ok(a),
            (None, Some(Node::Leaf(_))) => return Ok(b),
            (Some(Node::Leaf(left)), Some(Node::Leaf(right))) => {
                return match Disambiguate::cmp(left, right) {
                    Ordering::Less => Ok(b),
                    Ordering::Greater => Ok(a),
                    Ordering::Equal => Err(DisambiguationError(a, b)),
                };
            },
            _ => (),
        }

        let key = if a > b { [b, a] } else { [a, b] };

        // If the id pair is already merged (or is being merged), just return the id
        if let Some((_, merged)) = self.merges.iter().rev().find(|(k, _)| *k == key) {
            return Ok(*merged);
        }

        // Reserve the id for the merge and save it. Since the graph can contain loops,
        // this prevents us from trying to merge the same id pair in a loop, blowing up
        // the stack.
        let id = self.reserve();
        self.merges.push((key, id.get()));

        let [a, b] = key;

        let merged_rope = match (self.get(a), self.get(b)) {
            (Some(Node::Rope(rope)), _) => {
                let rope = rope.clone();

                self.merge_rope(rope, b)?
            },
            (_, Some(Node::Rope(rope))) => {
                let rope = rope.clone();

                self.merge_rope(rope, a)?
            },
            _ => None,
        };

        if let Some(rope) = merged_rope {
            return Ok(self.insert(id, rope));
        }

        let mut fork = self.fork_off(a);
        fork.merge(self.fork_off(b), self)?;

        let mut stack = vec![id.get()];

        // Flatten the fork
        while let Some(miss) = fork.miss {
            if stack.contains(&miss) {
                break;
            }
            stack.push(miss);

            let other = match self.get(miss) {
                Some(Node::Fork(other)) => other.clone(),
                Some(Node::Rope(other)) => other.clone().into_fork(self),
                _ => break,
            };
            match other.miss {
                Some(id) if self.get(id).is_none() => break,
                _ => (),
            }
            fork.miss = None;
            fork.merge(other, self)?;

        }

        Ok(self.insert(id, fork))
    }

    fn merge_rope(&mut self, rope: Rope, other: NodeId) -> Result<Option<Rope>>
    where
        Leaf: Disambiguate,
    {
        match self.get(other) {
            Some(Node::Fork(fork)) if rope.miss.is_none() => {
                // Count how many consecutive ranges in this rope would
                // branch into the fork that results in a loop.
                //
                // e.g.: for rope "foobar" and a looping fork [a-z]: 6
                let count = rope.pattern
                    .iter()
                    .take_while(|range| fork.contains(**range) == Some(other))
                    .count();

                let mut rope = match rope.split_at(count, self) {
                    Some(rope) => rope.miss_any(other),
                    None => return Ok(None),
                };

                rope.then = self.merge(rope.then, other)?;

                Ok(Some(rope))
            },
            Some(Node::Rope(other)) => {
                let (prefix, miss) = match rope.prefix(other) {
                    Some(pm) => pm,
                    None => return Ok(None),
                };

                let (a, b) = (rope, other.clone());

                let a = a.remainder(prefix.len(), self);
                let b = b.remainder(prefix.len(), self);

                let rope = Rope::new(prefix, self.merge(a, b)?).miss(miss);

                Ok(Some(rope))
            },
            Some(Node::Leaf(_)) | None => {
                if rope.miss.is_none() {
                    Ok(Some(rope.miss(other)))
                } else {
                    Ok(None)
                }
            },
            _ => Ok(None),
        }
    }

    pub fn fork_off(&mut self, id: NodeId) -> Fork
    where
        Leaf: Disambiguate,
    {
        match self.get(id) {
            Some(Node::Fork(fork)) => fork.clone(),
            Some(Node::Rope(rope)) => rope.clone().into_fork(self),
            Some(Node::Leaf(_)) | None => Fork::new().miss(id),
        }
    }

    pub fn nodes(&self) -> &[Option<Node<Leaf>>] {
        &self.nodes
    }

    /// Find all nodes that have no references and remove them.
    pub fn shake(&mut self, root: NodeId) {
        let mut filter = vec![false; self.nodes.len()];

        filter[root.get()] = true;

        self[root].shake(self, &mut filter);

        for (id, referenced) in filter.into_iter().enumerate() {
            if !referenced {
                self.nodes[id] = None;
            }
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node<Leaf>> {
        self.nodes.get(id.get())?.as_ref()
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node<Leaf>> {
        self.nodes.get_mut(id.get())?.as_mut()
    }
}

impl<Leaf> Index<NodeId> for Graph<Leaf> {
    type Output = Node<Leaf>;

    fn index(&self, id: NodeId) -> &Node<Leaf> {
        self.get(id).expect("Indexing into an empty node")
    }
}

impl<Leaf> IndexMut<NodeId> for Graph<Leaf> {
    fn index_mut(&mut self, id: NodeId) -> &mut Node<Leaf> {
        self.get_mut(id).expect("Indexing into an empty node")
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

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
    pub fn miss(&self) -> Option<NodeId> {
        match self {
            Node::Rope(rope) => rope.miss.first(),
            Node::Fork(fork) => fork.miss,
            Node::Leaf(_) => None,
        }
    }

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

    pub fn unwrap_leaf(&self) -> &Leaf {
        match self {
            Node::Fork(_) => panic!("Internal Error: called unwrap_leaf on a fork"),
            Node::Rope(_) => panic!("Internal Error: called unwrap_leaf on a rope"),
            Node::Leaf(leaf) => leaf,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn leaf_stack_size() {
        use std::mem::size_of;

        const WORD: usize = size_of::<usize>();
        const NODE: usize = size_of::<Node<()>>();

        assert!(NODE <= 6 * WORD, "Size of Node<()> is {} bytes!", NODE);
    }

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
        assert_eq!(graph.fork_off(rope), Fork::new().branch(b'r', NodeId::new(graph.nodes.len() - 1)));
        assert_eq!(graph.fork_off(fork), Fork::new().branch(b'!', leaf));
    }
}
