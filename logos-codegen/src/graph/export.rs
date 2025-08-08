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
        // Insert cannot be used since we also need a mut ref for get_unique
        #[allow(clippy::map_entry)]
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
                let escaped = (b as char).escape_default().flat_map(&escape);
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

struct Dot;

impl ExportFormat for Dot {
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

struct Mermaid;

impl ExportFormat for Mermaid {
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
        self.export_graph::<Dot>()
    }

    /// Writes the `Graph` to a mermaid file.
    pub fn get_mermaid(&self) -> Result<String, std::fmt::Error> {
        self.export_graph::<Mermaid>()
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
            Fmt::write_node(s, &link_id, &Fmt::fmt_range(range), NodeColor::Orange)?;
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
                    &format!("NOT {}", Fmt::fmt_range(self.pattern.first().unwrap())),
                    NodeColor::Red,
                )?;
                Fmt::write_link(s, &id, &link_id)?;
                Fmt::write_link(s, &link_id, ids.node(node))
            }
            Miss::Any(node) => {
                let link_id = ids.get_unique();
                Fmt::write_node(s, &link_id, "MISS", NodeColor::Red)?;
                Fmt::write_link(s, &id, &link_id)?;
                Fmt::write_link(s, &link_id, ids.node(node))
            }
            Miss::None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Fork, NodeId, Range, Rope};

    #[test]
    fn range_fmt_single_ascii_byte() {
        let r = Range {
            start: 0x6C,
            end: 0x6C,
        };
        assert_eq!(Dot::fmt_range(&r), "'l'");
        assert_eq!(Mermaid::fmt_range(&r), "'l'");
    }

    #[test]
    fn range_fmt_ascii_bytes() {
        let r = Range {
            start: 0x61,
            end: 0x7A,
        };
        assert_eq!(Dot::fmt_range(&r), "'a'..='z'");
        assert_eq!(Mermaid::fmt_range(&r), "'a'..='z'");
    }

    #[test]
    fn range_fmt_single_escaped_ascii_byte() {
        let r = Range {
            start: 0x22,
            end: 0x22,
        };
        assert_eq!(Dot::fmt_range(&r), "'\\\\\\\"'");
        assert_eq!(Mermaid::fmt_range(&r), "'\\\\&quot'");

        let r = Range {
            start: 0x5C,
            end: 0x5C,
        };
        assert_eq!(Dot::fmt_range(&r), "'\\\\\\\\'");
        assert_eq!(Mermaid::fmt_range(&r), "'\\\\\\\\'");
    }

    #[test]
    fn range_fmt_single_hex_byte() {
        let r = Range {
            start: 0x0A,
            end: 0x0A,
        };
        assert_eq!(Dot::fmt_range(&r), "0A");
        assert_eq!(Mermaid::fmt_range(&r), "0A");
    }

    #[test]
    fn range_fmt_hex_bytes() {
        let r = Range {
            start: 0x0A,
            end: 0x10,
        };
        assert_eq!(Dot::fmt_range(&r), "0A..=10");
        assert_eq!(Mermaid::fmt_range(&r), "0A..=10");
    }

    #[test]
    fn node_id_strings() {
        let mut ids = NodeIdStrings::new();
        let node_1 = ids.idx(1).to_owned();
        let node_2 = ids.idx(2).to_owned();
        let temp = ids.get_unique().to_owned();
        let node_1_again = ids.node(NodeId::new(1)).to_owned();
        assert_eq!(node_1, node_1_again);
        assert_ne!(node_2, temp);
        assert_ne!(node_1, node_2);
        assert_ne!(node_1, temp);
    }

    #[test]
    fn fork() {
        let n = Fork::new()
            .branch(
                Range {
                    start: 0x61,
                    end: 0x79,
                },
                NodeId::new(2),
            )
            .branch(
                Range {
                    start: 0x7A,
                    end: 0x7A,
                },
                NodeId::new(3),
            );

        let mut dot = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Dot>(&mut dot, &mut ids, 0).unwrap();
        assert_eq!(dot, "n0[label=\"Fork\",color=blue];n1[label=\"'a'..='y'\",color=orange];n0->n1;n1->n2;n3[label=\"'z'\",color=orange];n0->n3;n3->n4;");

        let mut mmd = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Mermaid>(&mut mmd, &mut ids, 0).unwrap();
        assert_eq!(mmd, "n0[\"Fork\"]\nstyle n0 stroke:#2962FF\nn1[\"'a'..='y'\"]\nstyle n1 stroke:#FF6D00\nn0-->n1\nn1-->n2\nn3[\"'z'\"]\nstyle n3 stroke:#FF6D00\nn0-->n3\nn3-->n4\n");
    }

    #[test]
    fn fork_with_miss() {
        let n = Fork::new()
            .branch(
                Range {
                    start: 0x61,
                    end: 0x7A,
                },
                NodeId::new(2),
            )
            .miss(NodeId::new(3));

        let mut dot = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Dot>(&mut dot, &mut ids, 0).unwrap();
        assert_eq!(dot, "n0[label=\"Fork\",color=blue];n1[label=\"'a'..='z'\",color=orange];n0->n1;n1->n2;n0->n3;");

        let mut mmd = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Mermaid>(&mut mmd, &mut ids, 0).unwrap();
        assert_eq!(mmd, "n0[\"Fork\"]\nstyle n0 stroke:#2962FF\nn1[\"'a'..='z'\"]\nstyle n1 stroke:#FF6D00\nn0-->n1\nn1-->n2\nn0-->n3\n");
    }

    #[test]
    fn rope() {
        let n = Rope::new("rope", NodeId::new(1));

        let mut dot = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Dot>(&mut dot, &mut ids, 0).unwrap();
        assert_eq!(dot, "n0[label=\"Rope\",color=blue];n1[label=\"'r'\",color=orange];n0->n1;n2[label=\"'o'\",color=orange];n1->n2;n3[label=\"'p'\",color=orange];n2->n3;n4[label=\"'e'\",color=orange];n3->n4;n4->n5;");

        let mut mmd = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Mermaid>(&mut mmd, &mut ids, 0).unwrap();
        assert_eq!(mmd, "n0[\"Rope\"]\nstyle n0 stroke:#2962FF\nn1[\"'r'\"]\nstyle n1 stroke:#FF6D00\nn0-->n1\nn2[\"'o'\"]\nstyle n2 stroke:#FF6D00\nn1-->n2\nn3[\"'p'\"]\nstyle n3 stroke:#FF6D00\nn2-->n3\nn4[\"'e'\"]\nstyle n4 stroke:#FF6D00\nn3-->n4\nn4-->n5\n");
    }

    #[test]
    fn rope_with_miss_first() {
        let n = Rope::new("ee", NodeId::new(1)).miss(NodeId::new(2));

        let mut dot = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Dot>(&mut dot, &mut ids, 0).unwrap();
        assert_eq!(dot, "n0[label=\"Rope\",color=blue];n1[label=\"'e'\",color=orange];n0->n1;n2[label=\"'e'\",color=orange];n1->n2;n2->n3;n4[label=\"NOT 'e'\",color=red];n0->n4;n4->n5;");

        let mut mmd = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Mermaid>(&mut mmd, &mut ids, 0).unwrap();
        assert_eq!(mmd, "n0[\"Rope\"]\nstyle n0 stroke:#2962FF\nn1[\"'e'\"]\nstyle n1 stroke:#FF6D00\nn0-->n1\nn2[\"'e'\"]\nstyle n2 stroke:#FF6D00\nn1-->n2\nn2-->n3\nn4[\"NOT 'e'\"]\nstyle n4 stroke:#D50000\nn0-->n4\nn4-->n5\n");
    }

    #[test]
    fn rope_with_miss_any() {
        let n = Rope::new("ee", NodeId::new(1)).miss_any(NodeId::new(2));

        let mut dot = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Dot>(&mut dot, &mut ids, 0).unwrap();
        assert_eq!(dot, "n0[label=\"Rope\",color=blue];n1[label=\"'e'\",color=orange];n0->n1;n2[label=\"'e'\",color=orange];n1->n2;n2->n3;n4[label=\"MISS\",color=red];n0->n4;n4->n5;");

        let mut mmd = String::new();
        let mut ids = NodeIdStrings::new();
        n.write_graph::<Mermaid>(&mut mmd, &mut ids, 0).unwrap();
        assert_eq!(mmd, "n0[\"Rope\"]\nstyle n0 stroke:#2962FF\nn1[\"'e'\"]\nstyle n1 stroke:#FF6D00\nn0-->n1\nn2[\"'e'\"]\nstyle n2 stroke:#FF6D00\nn1-->n2\nn2-->n3\nn4[\"MISS\"]\nstyle n4 stroke:#D50000\nn0-->n4\nn4-->n5\n");
    }
}
