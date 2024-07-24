use afl::fuzz;
use logos_codegen::{graph::{Graph, Node}, mir::Mir};

fn main() {
    fuzz!(|regex: String| {
        let mut graph = Graph::new();

        if let Ok(mir) = Mir::utf8(&regex) {
            let leaf = graph.push(Node::Leaf("LEAF"));
            let _ = graph.regex(mir, leaf);
        }
    });
}