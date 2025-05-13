use std::collections::HashMap;
use std::fmt::{Display, Write};
use crate::graph::{Fork, Graph, Node, NodeId, Range, Rope};
use crate::graph::rope::Miss;

struct DotNodeIds {
    next: u32,
    mappings: HashMap<usize, String>,
}

impl DotNodeIds {
    fn next(&mut self) -> String {
        let next = self.next;
        self.next += 1;
        format!("n{:x}", next)
    }

    fn idx(&mut self, id: usize) -> &str {
        if !self.mappings.contains_key(&id) {
            let next = self.next();
            self.mappings.insert(id, next);
        }
        self.mappings.get(&id).unwrap().as_str()
    }

    fn node(&mut self, node: NodeId) -> &str {
        self.idx(node.get())
    }
}

impl<Leaf: Display> Graph<Leaf> {
    /// Writes the `Graph` to a dot file.
    pub fn get_dot(&self) -> Result<String, std::fmt::Error> {
        let mut s = String::new();

        let entries = self
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.as_ref().map(|n| (i, n)));

        let mut ids = DotNodeIds {
            next: 0,
            mappings: HashMap::new(),
        };

        write!(s, "digraph Lexer{{")?;
        write!(s, "node[shape=box];")?;
        write!(s, "splines=ortho;")?;
        for (id, node) in entries {
            match node {
                Node::Fork(fork) => fork.write_dot(&mut s, &mut ids, id)?,
                Node::Rope(rope) => rope.write_dot(&mut s, &mut ids, id)?,
                Node::Leaf(leaf) => {
                    write!(s, "{}[label=\"{}\",color=green];", ids.idx(id), leaf)?;
                }
            }
        }
        write!(s, "}}")?;
        Ok(s)
    }
}

impl Fork {
    fn write_dot(&self, s: &mut String, ids: &mut DotNodeIds, id: usize) -> Result<(), std::fmt::Error> {
        let id = ids.idx(id).to_string();
        write!(s, "{}[label=\"Fork\",color=dodgerblue];", id)?;
        for (range, node) in self.branches() {
            let link_id = ids.next();
            write!(s, "{}[label=\"{}\",color=orange];", link_id, range.fmt_dot())?;
            write!(s, "{}->{};", id, link_id)?;
            write!(s, "{}->{};", link_id, ids.node(node))?;
        }
        if let Some(miss) = self.miss {
            write!(s, "{}->{};", id, ids.node(miss))?;
        }
        Ok(())
    }
}

impl Rope {
    fn write_dot(&self, s: &mut String, ids: &mut DotNodeIds, id: usize) -> Result<(), std::fmt::Error> {
        let id = ids.idx(id).to_string();
        write!(s, "{}[label=\"Rope\",color=blueviolet];", id)?;

        let mut previous = id.clone();
        for range in self.pattern.iter() {
            let link_id = ids.next();
            write!(s, "{}[label=\"{}\",color=orange];", link_id, range.fmt_dot())?;
            write!(s, "{}->{};", previous, link_id)?;
            previous = link_id;
        }
        write!(s, "{}->{};", previous, ids.node(self.then))?;

        match self.miss {
            Miss::First(node) => {
                let link_id = ids.next();
                write!(s, "{}[label=\"NOT {}\",color=red];", link_id, self.pattern.first().unwrap().fmt_dot())?;
                write!(s, "{}->{};", id, link_id)?;
                write!(s, "{}->{};", link_id, ids.node(node))
            }
            Miss::Any(node) => {
                write!(s, "{}->{};", id, ids.node(node))
            },
            Miss::None => Ok(()),
        }
    }
}

impl Range {
    fn fmt_dot(&self) -> String {
        if self.is_byte() && (0x20..0x7F).contains(&self.start) {
            let escaped = (self.start as char).escape_default().flat_map(|c| c.escape_default());
            format!("'{}'", escaped.collect::<String>())
        } else {
            self.to_string().escape_default().flat_map(|c| c.escape_default()).collect()
        }
    }
}
