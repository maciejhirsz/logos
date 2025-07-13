use crate::graph::{ByteClass, Graph, State, StateType};
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

    // /// Get the string assigned to a node from its id.
    // fn node(&mut self, node: NodeId) -> &str {
    //     self.idx(node.get())
    // }
}

enum NodeColor {
    Black,
    Red,
    Blue,
    Green,
    Orange,
}

impl NodeColor {
    fn fmt_dot(&self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Orange => "orange",
        }
    }

    fn fmt_mmd(&self) -> &'static str {
        match self {
            Self::Black => "#000000",
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

    fn fmt_range(bc: &ByteClass) -> String;
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

    fn fmt_range(bc: &ByteClass) -> String {
        // TODO: run ascii escape again?
        bc.to_string()
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

    fn fmt_range(bc: &ByteClass) -> String {
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
        // TODO
        // r.fmt_with_escape(escape_mmd)
        bc.to_string()
    }
}

impl Graph {
    /// Writes the `Graph` to a dot file.
    pub fn get_dot(&self) -> Result<String, std::fmt::Error> {
        self.export_graph::<Dot>()
    }

    /// Writes the `Graph` to a mermaid file.
    pub fn get_mermaid(&self) -> Result<String, std::fmt::Error> {
        self.export_graph::<Mermaid>()
    }

    fn export_graph<Fmt: ExportFormat>(&self) -> Result<String, std::fmt::Error> {
        fn format_state(state: &State, fancy: bool) -> String {
            let state_id = state.dfa_id.as_usize();
            match (fancy, state.context) {
                (true, None) => format!("State {}", state_id),
                (false, None) => format!("{}", state.dfa_id.as_usize()),
                (true, Some(leaf_id)) => format!("State {} (ctx {})", state_id, leaf_id.0),
                (false, Some(leaf_id)) => format!("{}_ctx{}", state_id, leaf_id.0),
            }
        }

        let mut s = String::new();

        Fmt::write_header(&mut s)?;

        for state in  self.get_states() {
            let data = self.get_state_data(&state);

            let id = format_state(&state, false);
            let label = format_state(&state, true);
            let color = if matches!(data.state_type, StateType::Accept(_)) {
                NodeColor::Green
            } else {
                NodeColor::Black
            };

            Fmt::write_node(&mut s, &id, &label, color)?;

            for (bc, to_state) in &data.normal {
                // Todo: label edge
                Fmt::write_link(&mut s, &id, &format_state(to_state, false));
            }
            if let Some(eoi_state) = &data.eoi {
                Fmt::write_link(&mut s, &id, &format_state(eoi_state, false));
            }

        }

        Fmt::write_footer(&mut s)?;

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use proc_macro2::Span;

    use crate::{graph::Config, leaf::Leaf, pattern::Pattern};

    use super::*;

    #[test]
    fn range_fmt_single_ascii_byte() {
        let r = ByteClass {
            ranges: vec![0x6C..=0x6C]
        };
        assert_snapshot!(Dot::fmt_range(&r), @"'l'");
        assert_snapshot!(Mermaid::fmt_range(&r), @"'l'");
    }

    #[test]
    fn range_fmt_ascii_bytes() {
        let r = ByteClass {
            ranges: vec![0x61..=0x7A]
        };
        assert_snapshot!(Dot::fmt_range(&r), @"'a'..='z'");
        assert_snapshot!(Mermaid::fmt_range(&r), @"'a'..='z'");
    }

    #[test]
    fn range_fmt_single_escaped_ascii_byte() {
        let r = ByteClass {
            ranges: vec![0x22..=0x22]
        };
        assert_snapshot!(Dot::fmt_range(&r), @"'\\\\\\\"'");
        assert_snapshot!(Mermaid::fmt_range(&r), @"'\\\\&quot'");

        let r = ByteClass {
            ranges: vec![0x5C..=0x5C]
        };
        assert_snapshot!(Dot::fmt_range(&r), @"'\\\\\\\\'");
        assert_snapshot!(Mermaid::fmt_range(&r), @"'\\\\\\\\'");
    }

    #[test]
    fn range_fmt_single_hex_byte() {
        let r = ByteClass {
            ranges: vec![0x0A..=0x0A]
        };
        assert_snapshot!(Dot::fmt_range(&r), @"0A");
        assert_snapshot!(Mermaid::fmt_range(&r), @"0A");
    }

    #[test]
    fn range_fmt_hex_bytes() {
        let r = ByteClass {
            ranges: vec![0x0A..=0x10]
        };
        assert_snapshot!(Dot::fmt_range(&r), @"0A..=10");
        assert_snapshot!(Mermaid::fmt_range(&r), @"0A..=10");
    }

    fn export_graphs(patterns: Vec<&str>) -> [String; 2] {
        let leaves = patterns.into_iter().map(|src| Leaf::new(Span::call_site(), Pattern::compile(src).expect("Unable to compile pattern"))).collect();

        let graph = Graph::new(leaves, Config::default()).expect("Unable to compile graph");
        let dot = graph.export_graph::<Dot>().unwrap();
        let mmd = graph.export_graph::<Mermaid>().unwrap();

        [dot, mmd]
    }

    #[test]
    fn fork() {
        let patterns = vec![
            "[a-y]",
            "z",
        ];

        let [dot, mmd] = export_graphs(patterns);
        assert_snapshot!(dot);
        assert_snapshot!(mmd);
    }

    #[test]
    fn rope() {
        let patterns = vec!["rope"];

        let [dot, mmd] = export_graphs(patterns);
        assert_snapshot!(dot);
        assert_snapshot!(mmd);
    }

    #[test]
    fn rope_with_miss_first() {
        let patterns = vec!["f(ee)?"];

        let [dot, mmd] = export_graphs(patterns);
        assert_snapshot!(dot);
        assert_snapshot!(mmd);
    }

    #[test]
    fn rope_with_miss_any() {
        let patterns = vec!["fe{,2}"];

        let [dot, mmd] = export_graphs(patterns);
        assert_snapshot!(dot);
        assert_snapshot!(mmd);
    }
}
