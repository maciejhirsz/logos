use crate::graph::{Graph, Fork, NodeId};

#[derive(PartialEq, Clone)]
pub struct Rope {
    pub pattern: Vec<u8>,
    pub then: NodeId,
    pub miss: Miss,
}

/// Because Ropes could potentially fail a match mid-pattern,
/// a regular `Option` is not sufficient here.
#[derive(PartialEq, Clone, Copy)]
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
        // P: Into<Vec<u8>>,
        P: AsRef<[u8]>,
    {
        Rope {
            pattern: pattern.as_ref().to_vec(),
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
                    pattern: self.pattern[1..].to_vec(),
                    then: self.then,
                    miss: self.miss.any().into(),
                })
            },
        };

        Fork::new().branch(self.pattern[0], then).miss(self.miss.first())
    }

    pub fn prefix(&self, other: &Self) -> Option<Vec<u8>> {
        let count = self.pattern
            .iter()
            .zip(&other.pattern)
            .take_while(|(a, b)| a == b)
            .count();

        match count {
            0 => None,
            _ => Some(self.pattern[..count].to_vec()),
        }
    }

    pub fn remainder<T>(mut self, at: usize, graph: &mut Graph<T>) -> NodeId {
        self.pattern = self.pattern[at..].to_vec();

        match self.pattern.len() {
            0 => graph.push_miss(self.then, self.miss.any()),
            _ => graph.push(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::NodeBody;
    use pretty_assertions::assert_eq;

    #[test]
    fn fork_off() {
        let mut graph = Graph::new();

        let token = graph.push(NodeBody::Leaf("FOOBAR"));
        let rope = Rope::new("foobar", token);

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'f', 1));
        assert_eq!(
            graph[1].body,
            NodeBody::Rope(
                Rope::new("oobar", token),
            ),
        );
    }

    #[test]
    fn fork_off_one_byte() {
        let mut graph = Graph::new();

        let token = graph.push(NodeBody::Leaf("FOOBAR"));
        let rope = Rope::new("!", token);

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'!', 0));
    }

    #[test]
    fn fork_off_miss_any() {
        let mut graph = Graph::new();

        let token = graph.push(NodeBody::Leaf("LIFE"));
        let rope = Rope::new("42", token).miss(Miss::Any(42));

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'4', 1).miss(42));
        assert_eq!(
            graph[1].body,
            NodeBody::Rope(
                Rope::new("2", token).miss(42),
            ),
        );
    }

    #[test]
    fn fork_off_miss_first() {
        let mut graph = Graph::new();

        let token = graph.push(NodeBody::Leaf("LIFE"));
        let rope = Rope::new("42", token).miss(Miss::First(42));

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'4', 1).miss(42));
        assert_eq!(
            graph[1].body,
            NodeBody::Rope(
                Rope::new("2", token),
            ),
        );
    }
}