use crate::graph::{Graph, Fork, Pattern, NodeId};

#[derive(PartialEq)]
pub struct Rope {
    pub pattern: Pattern,
    pub then: NodeId,
    pub miss: Option<NodeId>,
}

impl Rope {
    pub fn new<P>(pattern: P, then: NodeId) -> Self
    where
        P: Into<Pattern>,
    {
        Rope {
            pattern: pattern.into(),
            then,
            miss: None,
        }
    }

    pub fn miss<M>(mut self, miss: M) -> Self
    where
        M: Into<Option<NodeId>>,
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
                graph.put(|_| Rope {
                    pattern: Pattern(self.pattern[1..].to_vec()),
                    then: self.then,
                    miss: self.miss,
                })
            },
        };

        Fork::new().branch(self.pattern[0], then).miss(self.miss)
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

        let token = graph.put(|_| NodeBody::Leaf("FOOBAR"));
        let rope = Rope::new(b"foobar", token);

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'f', 1));
        assert_eq!(
            graph[1].body,
            NodeBody::Rope(
                Rope::new(b"oobar", token),
            ),
        );
    }

    #[test]
    fn fork_off_one_byte() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("FOOBAR"));
        let rope = Rope::new(b"!", token);

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'!', 0));
    }

    #[test]
    fn fork_off_miss_value() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("LIFE"));
        let rope = Rope::new(b"42", token).miss(42);

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new().branch(b'4', 1).miss(42));
        assert_eq!(
            graph[1].body,
            NodeBody::Rope(
                Rope::new(b"2", token).miss(42),
            ),
        );
    }
}