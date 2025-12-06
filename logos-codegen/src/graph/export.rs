use crate::graph::{Graph, StateType};
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

enum NodeShape {
    Rectangle,
    Rhombus,
}

trait ExportFormat {
    fn write_header(s: &mut String) -> std::fmt::Result;

    fn write_footer(s: &mut String) -> std::fmt::Result;

    fn write_node(
        s: &mut String,
        id: &str,
        label: &str,
        color: NodeColor,
        shape: NodeShape,
    ) -> std::fmt::Result;

    fn write_link(s: &mut String, from: &str, to: &str) -> std::fmt::Result;

    fn escape(s: String) -> String;
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

    fn write_node(
        s: &mut String,
        id: &str,
        label: &str,
        color: NodeColor,
        shape: NodeShape,
    ) -> std::fmt::Result {
        let shape_str = match shape {
            NodeShape::Rectangle => "box",
            NodeShape::Rhombus => "diamond",
        };
        writeln!(
            s,
            "{id}[label=\"{label}\",color={},shape={}];",
            color.fmt_dot(),
            shape_str
        )
    }

    fn write_link(s: &mut String, from: &str, to: &str) -> std::fmt::Result {
        writeln!(s, "{from}->{to};")
    }

    fn escape(s: String) -> String {
        s.escape_default().to_string()
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

    fn write_node(
        s: &mut String,
        id: &str,
        label: &str,
        color: NodeColor,
        shape: NodeShape,
    ) -> std::fmt::Result {
        match shape {
            NodeShape::Rectangle => writeln!(s, "{id}[\"{label}\"]")?,
            NodeShape::Rhombus => writeln!(s, "{id}{{\"{label}\"}}")?,
        }
        writeln!(s, "style {id} stroke:{}", color.fmt_mmd())
    }

    fn write_link(s: &mut String, from: &str, to: &str) -> std::fmt::Result {
        writeln!(s, "{from}-->{to}")
    }

    fn escape(s: String) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                '"' => {
                    let _ = result.write_str("&quot");
                }
                '\\' => {
                    let _ = result.write_str("\\\\");
                }
                '\n' => {
                    let _ = result.write_str("<br>");
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
        let shape_ids = self
            .iter_states()
            .map(|state| format!("n{}", state.0))
            .collect::<Vec<_>>();
        let shape_names = self
            .iter_states()
            .map(|state| {
                let state_id = state.0;
                let rendered = match self.states[state_id].state_type {
                    StateType {
                        early: Some(leaf_id),
                        ..
                    } => format!("State {state_id}\nearly({})", leaf_id.0),
                    StateType {
                        accept: Some(leaf_id),
                        ..
                    } => format!("State {state_id}\nlate({})", leaf_id.0),
                    _ => format!("State {state_id}"),
                };
                Fmt::escape(rendered)
            })
            .collect::<Vec<_>>();

        let mut s = String::new();

        Fmt::write_header(&mut s)?;

        for state in self.iter_states() {
            let data = self.get_state(state);

            let id = &shape_ids[state.0];
            let label = &shape_names[state.0];
            let color = if data.state_type.early_or_accept().is_some() {
                NodeColor::Green
            } else {
                NodeColor::Black
            };

            Fmt::write_node(&mut s, id, label, color, NodeShape::Rectangle)?;

            let normal_edges = data
                .normal
                .iter()
                .map(|(bc, to_state)| (Fmt::escape(format!("{:#}", bc)), to_state));

            let eoi_edge = data.eoi.as_ref().map(|state| (String::from("EOI"), state));

            for (label, to_state) in normal_edges.chain(eoi_edge) {
                let to_id = &shape_ids[to_state.0];
                let edge_id = format!("e{}{}", id, to_id);
                Fmt::write_node(
                    &mut s,
                    &edge_id,
                    &label,
                    NodeColor::Black,
                    NodeShape::Rhombus,
                )?;
                Fmt::write_link(&mut s, id, &edge_id)?;
                Fmt::write_link(&mut s, &edge_id, to_id)?;
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

    use crate::{
        graph::{ByteClass, Config},
        leaf::Leaf,
        pattern::Pattern,
    };

    use super::*;

    fn fmt_range<Fmt: ExportFormat>(bc: &ByteClass) -> String {
        Fmt::escape(format!("{:#}", bc))
    }

    #[test]
    fn range_fmt_single_ascii_byte() {
        let r = ByteClass {
            ranges: vec![0x6C..=0x6C],
        };
        assert_snapshot!(fmt_range::<Dot>(&r), @"l");
        assert_snapshot!(fmt_range::<Mermaid>(&r), @"l");
    }

    #[test]
    fn range_fmt_ascii_bytes() {
        let r = ByteClass {
            ranges: vec![0x61..=0x7A],
        };
        assert_snapshot!(fmt_range::<Dot>(&r), @"a..=z");
        assert_snapshot!(fmt_range::<Mermaid>(&r), @"a..=z");
    }

    #[test]
    fn range_fmt_single_escaped_ascii_byte() {
        let r = ByteClass {
            ranges: vec![0x22..=0x22],
        };
        assert_snapshot!(fmt_range::<Dot>(&r), @r###"\\\""###);
        assert_snapshot!(fmt_range::<Mermaid>(&r), @r###"\\&quot"###);

        let r = ByteClass {
            ranges: vec![0x5C..=0x5C],
        };
        assert_snapshot!(fmt_range::<Dot>(&r), @r###"\\\\"###);
        assert_snapshot!(fmt_range::<Mermaid>(&r), @r###"\\\\"###);
    }

    #[test]
    fn range_fmt_single_hex_byte() {
        let r = ByteClass {
            ranges: vec![0x0A..=0x0A],
        };
        assert_snapshot!(fmt_range::<Dot>(&r), @r###"\\n"###);
        assert_snapshot!(fmt_range::<Mermaid>(&r), @r###"\\n"###);
    }

    #[test]
    fn range_fmt_hex_bytes() {
        let r = ByteClass {
            ranges: vec![0x0A..=0x10],
        };
        assert_snapshot!(fmt_range::<Dot>(&r), @r###"\\n..=\\x10"###);
        assert_snapshot!(fmt_range::<Mermaid>(&r), @r###"\\n..=\\x10"###);
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

        let config = Config { utf8_mode: true };
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
