use crate::graph::{Graph, Fork, NodeId, NodeBody};

#[cfg_attr(test, derive(PartialEq))]
pub struct Rope {
    pub bytes: Vec<u8>,
    pub then: NodeId,
    pub miss: Option<NodeId>,
}

impl Rope {
    pub fn fork_off<T>(&self, graph: &mut Graph<T>) -> Fork {
        // The new fork will lead to a new rope,
        // or the old target if no new rope was created
        let then = match self.bytes.len() {
            0 => panic!("Logos Internal Error: Trying to fork a Rope without bytes left"),
            1 => self.then,
            _ => {
                graph.put(|_| Rope {
                    bytes: self.bytes[1..].to_vec(),
                    then: self.then,
                    miss: self.miss,
                })
            },
        };

        Fork::new(self.miss).branch(self.bytes[0], then)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn fork_off() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("FOOBAR"));
        let rope = Rope {
            bytes: b"foobar".to_vec(),
            then: token,
            miss: None,
        };

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new(None).branch(b'f', 1));
        assert_eq!(graph[1].body, NodeBody::Rope(Rope {
            bytes: b"oobar".to_vec(),
            then: token,
            miss: None,
        }));
    }

    #[test]
    fn fork_off_one_byte() {
        let mut graph = Graph::new();

        let token = graph.put(|_| NodeBody::Leaf("FOOBAR"));
        let rope = Rope {
            bytes: vec![b'!'],
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
            bytes: b"42".to_vec(),
            then: token,
            miss: Some(42),
        };

        let fork = rope.fork_off(&mut graph);

        assert_eq!(token, 0);
        assert_eq!(fork, Fork::new(42).branch(b'4', 1));
        assert_eq!(graph[1].body, NodeBody::Rope(Rope {
            bytes: b"2".to_vec(),
            then: token,
            miss: Some(42),
        }));
    }
}