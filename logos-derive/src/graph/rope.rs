use std::ops::Deref;

use crate::graph::{Graph, Range, Fork, NodeId};

#[derive(PartialEq, Clone, Hash)]
pub struct Rope {
    pub pattern: Pattern,
    pub then: NodeId,
    pub miss: Miss,
}

#[derive(PartialEq, Clone, Hash)]
pub struct Pattern(pub Vec<Range>);

impl Deref for Pattern {
    type Target = [Range];

    fn deref(&self) -> &[Range] {
        &self.0
    }
}

/// Because Ropes could potentially fail a match mid-pattern,
/// a regular `Option` is not sufficient here.
#[derive(PartialEq, Clone, Copy, Hash)]
pub enum Miss {
    /// Same as Option::None, error on fail
    None,
    /// Jump to id if first byte does not match, fail on partial match
    First(NodeId),
    /// Jump to id on partial or empty match
    Any(NodeId),
}

impl Miss {
    pub fn any(self) -> Option<NodeId> {
        match self {
            Miss::Any(id) => Some(id),
            _ => None,
        }
    }

    pub fn first(self) -> Option<NodeId> {
        match self {
            Miss::First(id) | Miss::Any(id) => Some(id),
            _ => None,
        }
    }
}

impl From<Option<NodeId>> for Miss {
    fn from(miss: Option<NodeId>) -> Self {
        match miss {
            Some(id) => Miss::First(id),
            None => Miss::None,
        }
    }
}

impl From<NodeId> for Miss {
    fn from(id: NodeId) -> Self {
        Miss::First(id)
    }
}

impl Rope {
    pub fn new<P>(pattern: P, then: NodeId) -> Self
    where
        P: Into<Pattern>,
    {
        Rope {
            pattern: pattern.into(),
            then,
            miss: Miss::None,
        }
    }

    pub fn miss<M>(mut self, miss: M) -> Self
    where
        M: Into<Miss>,
    {
        self.miss = miss.into();
        self
    }

    pub fn fork_off<T>(&self, graph: &mut Graph<T>) -> Fork {
        // The new fork will lead to a new rope,
        // or the old target if no new rope was created
        let then = match self.pattern.len() {
            0 => panic!("Logos Internal Error: Trying to fork a Rope without bytes left"),
            1 => self.then,
            _ => {
                graph.push(Rope {
                    pattern: self.pattern[1..].into(),
                    then: self.then,
                    miss: self.miss.any().into(),
                })
            },
        };

        Fork::new().branch(self.pattern[0], then).miss(self.miss.first())
    }

    pub fn prefix(&self, other: &Self) -> Option<Pattern> {
        let count = self.pattern
            .iter()
            .zip(other.pattern.iter())
            .take_while(|(a, b)| a == b)
            .count();

        match count {
            0 => None,
            _ => Some(self.pattern[..count].into()),
        }
    }

    pub fn remainder<T>(mut self, at: usize, graph: &mut Graph<T>) -> NodeId {
        self.pattern = self.pattern[at..].into();

        match self.pattern.len() {
            0 => graph.push_miss(self.then, self.miss.any()),
            _ => graph.push(self),
        }
    }

    pub fn shake<T>(&self, graph: &Graph<T>, filter: &mut [bool]) {
        if let Some(id) = self.miss.first() {
            if !filter[id] {
                filter[id] = true;
                graph[id].shake(graph, filter);
            }
        }

        if !filter[self.then] {
            filter[self.then] = true;
            graph[self.then].shake(graph, filter);
        }
    }
}

impl<T> From<&[T]> for Pattern
where
    T: Into<Range> + Copy,
{
    fn from(slice: &[T]) -> Self {
        Pattern(slice.iter().copied().map(Into::into).collect())
    }
}

impl<T> From<Vec<T>> for Pattern
where
    T: Into<Range>,
{
    fn from(vec: Vec<T>) -> Self {
        Pattern(vec.into_iter().map(Into::into).collect())
    }
}

impl From<&str> for Pattern {
    fn from(slice: &str) -> Self {
        slice.as_bytes().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Node;
    use pretty_assertions::assert_eq;

    #[test]
    fn fork_off() {
        let mut graph = Graph::new();

        let token = graph.push(Node::Leaf("FOOBAR"));
        let rope = Rope::new("foobar", token);

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'f', 1));
        assert_eq!(graph[1], Rope::new("oobar", token));
    }

    #[test]
    fn fork_off_one_byte() {
        let mut graph = Graph::new();

        let token = graph.push(Node::Leaf("FOOBAR"));
        let rope = Rope::new("!", token);

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'!', 0));
    }

    #[test]
    fn fork_off_miss_any() {
        let mut graph = Graph::new();

        let token = graph.push(Node::Leaf("LIFE"));
        let rope = Rope::new("42", token).miss(Miss::Any(42));

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'4', 1).miss(42));
        assert_eq!(graph[1], Rope::new("2", token).miss(42));
    }

    #[test]
    fn fork_off_miss_first() {
        let mut graph = Graph::new();

        let token = graph.push(Node::Leaf("LIFE"));
        let rope = Rope::new("42", token).miss(Miss::First(42));

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'4', 1).miss(42));
        assert_eq!(graph[1], Rope::new("2", token));
    }
}