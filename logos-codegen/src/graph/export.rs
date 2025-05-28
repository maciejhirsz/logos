use crate::graph::rope::Miss;
use crate::graph::{Fork, Graph, Node, NodeId, Range, Rope};
use std::collections::HashMap;
use std::fmt::{Display, Write};

/// Helps assign short strings to node ids while avoiding name collisions.
struct NodeIdStrings {
    next: u32,
    mappings: HashMap<usize, String>,
}

impl NodeIdStrings {
    fn new() -> Self {
        Self {
            next: 0,
            mappings: HashMap::new(),
        }
    }
    
    /// Get a unique string for a node.
    fn get_unique(&mut self) -> String {
        let next = self.next;
        self.next += 1;
        format!("n{:x}", next)
    }

    /// Get the string assigned to a node from its index.
    fn idx(&mut self, id: usize) -> &str {
        if !self.mappings.contains_key(&id) {
            let next = self.get_unique();
            self.mappings.insert(id, next);
        }
        self.mappings.get(&id).unwrap().as_str()
    }

    /// Get the string assigned to a node from its id.
    fn node(&mut self, node: NodeId) -> &str {
        self.idx(node.get())
    }
}

enum NodeColor {
    Red,
    Blue,
    Green,
    Orange,
}

impl NodeColor {
    fn fmt_dot(&self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Orange => "orange",
        }
    }

    fn fmt_mmd(&self) -> &'static str {
        match self {
            Self::Red => "#D50000",
            Self::Blue => "#2962FF",
            Self::Green => "#00C853",
            Self::Orange => "#FF6D00",
        }
    }
}

trait ExportFormat {
    fn write_header(s: &mut String) -> std::fmt::Result;

    fn write_footer(s: &mut String) -> std::fmt::Result;

    fn write_node(s: &mut String, id: &str, label: &str, color: NodeColor) -> std::fmt::Result;

    fn write_link(s: &mut String, from: &str, to: &str) -> std::fmt::Result;

    fn fmt_range(r: &Range) -> String;
}

impl Range {
    fn fmt_with_escape<I: Iterator<Item = char>>(&self, escape: impl Fn(char) -> I) -> String {
        let fmt_byte = |b: u8| -> String {
            if (0x20..0x7F).contains(&b) {
                let escaped = (b as char).escape_default().flat_map(|c| escape(c));
                format!("'{}'", escaped.collect::<String>())
            } else {
                format!("{:02X}", b)
            }
        };

        if let Some(b) = self.as_byte() {
            fmt_byte(b)
        } else {
            format!("{}..={}", fmt_byte(self.start), fmt_byte(self.end))
        }
    }
}

struct DOT;

impl ExportFormat for DOT {
    fn write_header(s: &mut String) -> std::fmt::Result {
        write!(s, "digraph {{")?;
        write!(s, "node[shape=box];")?;
        write!(s, "splines=ortho;")
    }

    fn write_footer(s: &mut String) -> std::fmt::Result {
        write!(s, "}}")
    }

    fn write_node(s: &mut String, id: &str, label: &str, color: NodeColor) -> std::fmt::Result {
        write!(s, "{}[label=\"{}\",color={}];", id, label, color.fmt_dot())
    }

    fn write_link(s: &mut String, from: &str, to: &str) -> std::fmt::Result {
        write!(s, "{}->{};", from, to)
    }

    fn fmt_range(r: &Range) -> String {
        r.fmt_with_escape(|c| c.escape_default())
    }
}

struct MMD;

impl ExportFormat for MMD {
    fn write_header(s: &mut String) -> std::fmt::Result {
        writeln!(s, "flowchart TB")
    }

    fn write_footer(_s: &mut String) -> std::fmt::Result {
        Ok(())
    }

    fn write_node(s: &mut String, id: &str, label: &str, color: NodeColor) -> std::fmt::Result {
        writeln!(s, "{}[\"{}\"]", id, label)?;
        writeln!(s, "style {} stroke:{}", id, color.fmt_mmd())
    }

    fn write_link(s: &mut String, from: &str, to: &str) -> std::fmt::Result {
        writeln!(s, "{}-->{}", from, to)
    }

    fn fmt_range(r: &Range) -> String {
        fn escape_mmd(c: char) -> impl Iterator<Item = char> {
            enum Iter {
                Char(Option<char>),
                Str(std::str::Chars<'static>),
            }

            impl Iterator for Iter {
                type Item = char;

                fn next(&mut self) -> Option<Self::Item> {
                    match self {
                        Iter::Char(c) => c.take(),
                        Iter::Str(cs) => cs.next(),
                    }
                }
            }
            match c {
                '"' => Iter::Str("&quot".chars()),
                '\\' => Iter::Str("\\\\".chars()),
                _ => Iter::Char(Some(c)),
            }
        }
        r.fmt_with_escape(escape_mmd)
    }
}

impl<Leaf: Display> Graph<Leaf> {
    /// Writes the `Graph` to a dot file.
    pub fn get_dot(&self) -> Result<String, std::fmt::Error> {
        self.export_graph::<DOT>()
    }

    /// Writes the `Graph` to a mermaid file.
    pub fn get_mmd(&self) -> Result<String, std::fmt::Error> {
        self.export_graph::<MMD>()
    }

    fn export_graph<Fmt: ExportFormat>(&self) -> Result<String, std::fmt::Error> {
        let mut s = String::new();

        let entries = self
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.as_ref().map(|n| (i, n)));

        let mut ids = NodeIdStrings::new();

        Fmt::write_header(&mut s)?;
        for (id, node) in entries {
            match node {
                Node::Fork(fork) => fork.write_graph::<Fmt>(&mut s, &mut ids, id)?,
                Node::Rope(rope) => rope.write_graph::<Fmt>(&mut s, &mut ids, id)?,
                Node::Leaf(leaf) => {
                    Fmt::write_node(&mut s, ids.idx(id), &leaf.to_string(), NodeColor::Green)?;
                }
            }
        }
        Fmt::write_footer(&mut s)?;

        Ok(s)
    }
}

impl Fork {
    fn write_graph<Fmt: ExportFormat>(
        &self,
        s: &mut String,
        ids: &mut NodeIdStrings,
        id: usize,
    ) -> Result<(), std::fmt::Error> {
        let id = ids.idx(id).to_string();
        Fmt::write_node(s, &id, "Fork", NodeColor::Blue)?;
        for (range, node) in self.branches() {
            let link_id = ids.get_unique();
            Fmt::write_node(s, &link_id, &Fmt::fmt_range(&range), NodeColor::Orange)?;
            Fmt::write_link(s, &id, &link_id)?;
            Fmt::write_link(s, &link_id, ids.node(node))?;
        }
        if let Some(miss) = self.miss {
            Fmt::write_link(s, &id, ids.node(miss))?;
        }
        Ok(())
    }
}

impl Rope {
    fn write_graph<Fmt: ExportFormat>(
        &self,
        s: &mut String,
        ids: &mut NodeIdStrings,
        id: usize,
    ) -> Result<(), std::fmt::Error> {
        let id = ids.idx(id).to_owned();
        Fmt::write_node(s, &id, "Rope", NodeColor::Blue)?;

        let mut previous = id.clone();
        for range in self.pattern.iter() {
            let link_id = ids.get_unique();
            Fmt::write_node(s, &link_id, &Fmt::fmt_range(&range), NodeColor::Orange)?;
            Fmt::write_link(s, &previous, &link_id)?;
            previous = link_id;
        }
        Fmt::write_link(s, &previous, ids.node(self.then))?;

        match self.miss {
            Miss::First(node) => {
                let link_id = ids.get_unique();
                Fmt::write_node(
                    s,
                    &link_id,
                    &format!("NOT {}", Fmt::fmt_range(&self.pattern.first().unwrap())),
                    NodeColor::Red,
                )?;
                Fmt::write_link(s, &id, &link_id)?;
                Fmt::write_link(s, &link_id, ids.node(node))
            }
            Miss::Any(node) => Fmt::write_link(s, &id, ids.node(node)),
            Miss::None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t() {}
}
