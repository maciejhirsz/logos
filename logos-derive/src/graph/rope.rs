use crate::graph::{Graph, Fork, Pattern, NodeId};

#[cfg_attr(test, derive(PartialEq))]
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

        Fork::new(self.miss).branch(self.pattern[0], then)
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
        let rope = Rope {
            pattern: Pattern::from(b"foobar"),
            then: token,
            miss: None,
        };

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new(None).branch(b'f', 1));
        assert_eq!(graph[1].body, NodeBody::Rope(Rope {
            pattern: b"oobar".iter().into(),
            then: token,
            miss: None,
        }));
    }

    #[test]
    fn fork_off_one_byte() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("FOOBAR"));
        let rope = Rope {
            pattern: b"!".into(),
            then: token,
            miss: None,
        };

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new(None).branch(b'!', 0));
    }

    #[test]
    fn fork_off_miss_value() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("LIFE"));
        let rope = Rope {
            pattern: b"42".into(),
            then: token,
            miss: Some(42),
        };

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new(42).branch(b'4', 1));
        assert_eq!(graph[1].body, NodeBody::Rope(Rope {
            pattern: b"2".into(),
            then: token,
            miss: Some(42),
        }));
    }
}