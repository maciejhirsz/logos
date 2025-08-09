use crate::graph::{ByteClass, Graph, State};
use std::fmt::Write;

enum NodeColor {
    Black,
    Green,
}

impl NodeColor {
    fn fmt_dot(&self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Green => "green",
        }
    }

    fn fmt_mmd(&self) -> &'static str {
        match self {
            Self::Black => "#000000",
            Self::Green => "#00C853",
        }
    }
}

trait ExportFormat {
    fn write_header(s: &mut String) -> std::fmt::Result;

    fn write_footer(s: &mut String) -> std::fmt::Result;

    fn write_node(s: &mut String, id: &str, label: &str, color: NodeColor) -> std::fmt::Result;

    fn write_link(s: &mut String, from: &str, to: &str, label: &str) -> std::fmt::Result;

    fn fmt_range(bc: &ByteClass) -> String;
}

struct Dot;

impl ExportFormat for Dot {
    fn write_header(s: &mut String) -> std::fmt::Result {
        writeln!(s, "digraph {{")?;
        writeln!(s, "node[shape=box];")?;
        writeln!(s, "splines=ortho;")
    }

    fn write_footer(s: &mut String) -> std::fmt::Result {
        writeln!(s, "}}")
    }

    fn write_node(s: &mut String, id: &str, label: &str, color: NodeColor) -> std::fmt::Result {
        writeln!(s, "{id}[label=\"{label}\",color={}];", color.fmt_dot())
    }

    fn write_link(s: &mut String, from: &str, to: &str, label: &str) -> std::fmt::Result {
        writeln!(s, "{from}->{to} [label=\"{label}\"];")
    }

    fn fmt_range(bc: &ByteClass) -> String {
        bc.to_string().escape_default().to_string()
    }
}

struct Mermaid;

impl ExportFormat for Mermaid {
    // TODO: This should really use the mermaid diagram type
    // `stateDiagram` instead. since it more closely aligns with what we are representing.
    fn write_header(s: &mut String) -> std::fmt::Result {
        writeln!(s, "flowchart TB")
    }

    fn write_footer(_s: &mut String) -> std::fmt::Result {
        Ok(())
    }

    fn write_node(s: &mut String, id: &str, label: &str, color: NodeColor) -> std::fmt::Result {
        writeln!(s, "{id}[\"{label}\"]")?;
        writeln!(s, "style {id} stroke:{}", color.fmt_mmd())
    }

    fn write_link(s: &mut String, from: &str, to: &str, label: &str) -> std::fmt::Result {
        writeln!(s, "{from}-->|\"{label}\"|{to}")
    }

    fn fmt_range(bc: &ByteClass) -> String {
        let mut result = String::new();
        for c in bc.to_string().chars() {
            match c {
                '"' => {
                    let _ = result.write_str("&quot");
                }
                '\\' => {
                    let _ = result.write_str("\\\\");
                }
                _ => result.push(c),
            }
        }

        result
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
                (false, None) => format!("n{}", state.dfa_id.as_usize()),
                (true, Some(leaf_id)) => format!("State {} (ctx {})", state_id, leaf_id.0),
                (false, Some(leaf_id)) => format!("n{}ctx{}", state_id, leaf_id.0),
            }
        }

        let mut s = String::new();

        Fmt::write_header(&mut s)?;

        let mut states = self.get_states().collect::<Vec<_>>();
        // Sort for repeatability (not dependent on hashmap iteration order)
        states.sort_unstable();
        for state in states {
            let data = self.get_state_data(&state);

            let id = format_state(&state, false);
            let label = format_state(&state, true);
            let color = if data.state_type.early_accept.is_some() || data.state_type.accept.is_some() {
                NodeColor::Green
            } else {
                NodeColor::Black
            };

            Fmt::write_node(&mut s, &id, &label, color)?;

            for (bc, to_state) in &data.normal {
                let to_id = format_state(to_state, false);
                let range = Fmt::fmt_range(bc);
                Fmt::write_link(&mut s, &id, &to_id, &range)?;
            }
            if let Some(eoi_state) = &data.eoi {
                let eoi_id = format_state(eoi_state, false);
                Fmt::write_link(&mut s, &id, &eoi_id, "EOI")?;
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
            ranges: vec![0x6C..=0x6C],
        };
        assert_snapshot!(Dot::fmt_range(&r), @"l");
        assert_snapshot!(Mermaid::fmt_range(&r), @"l");
    }

    #[test]
    fn range_fmt_ascii_bytes() {
        let r = ByteClass {
            ranges: vec![0x61..=0x7A],
        };
        assert_snapshot!(Dot::fmt_range(&r), @"a..=z");
        assert_snapshot!(Mermaid::fmt_range(&r), @"a..=z");
    }

    #[test]
    fn range_fmt_single_escaped_ascii_byte() {
        let r = ByteClass {
            ranges: vec![0x22..=0x22],
        };
        assert_snapshot!(Dot::fmt_range(&r), @r###"\\\""###);
        assert_snapshot!(Mermaid::fmt_range(&r), @r###"\\&quot"###);

        let r = ByteClass {
            ranges: vec![0x5C..=0x5C],
        };
        assert_snapshot!(Dot::fmt_range(&r), @r###"\\\\"###);
        assert_snapshot!(Mermaid::fmt_range(&r), @r###"\\\\"###);
    }

    #[test]
    fn range_fmt_single_hex_byte() {
        let r = ByteClass {
            ranges: vec![0x0A..=0x0A],
        };
        assert_snapshot!(Dot::fmt_range(&r), @r###"\\n"###);
        assert_snapshot!(Mermaid::fmt_range(&r), @r###"\\n"###);
    }

    #[test]
    fn range_fmt_hex_bytes() {
        let r = ByteClass {
            ranges: vec![0x0A..=0x10],
        };
        assert_snapshot!(Dot::fmt_range(&r), @r###"\\n..=\\x10"###);
        assert_snapshot!(Mermaid::fmt_range(&r), @r###"\\n..=\\x10"###);
    }

    fn export_graphs(patterns: Vec<&str>) -> [String; 2] {
        let leaves = patterns
            .into_iter()
            .map(|src| {
                Leaf::new(
                    Span::call_site(),
                    Pattern::compile(false, src, src.to_string(), true, false)
                        .expect("Unable to compile pattern"),
                )
            })
            .collect();

        let config = Config {
            prio_over_length: false,
            utf8_mode: true,
        };
        let graph = Graph::new(leaves, config).expect("Unable to compile graph");
        let dot = graph.export_graph::<Dot>().unwrap();
        let mmd = graph.export_graph::<Mermaid>().unwrap();

        [dot, mmd]
    }

    #[test]
    fn fork() {
        let patterns = vec!["[a-y]", "z"];

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
        let patterns = vec!["fe{0,2}"];

        let [dot, mmd] = export_graphs(patterns);
        assert_snapshot!(dot);
        assert_snapshot!(mmd);
    }
}
