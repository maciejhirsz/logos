use std::collections::BTreeMap as Map;

use proc_macro2::{TokenStream, Span};
use quote::{quote, TokenStreamExt};
use syn::Ident;

use crate::graph::{Graph, Node, NodeId, Fork, Rope};
use crate::token::Token;

pub struct Generator<'a> {
    name: &'a Ident,
    root: NodeId,
    err_id: NodeId,
    end_id: NodeId,
    gotos: Map<NodeId, Ident>,
}

impl<'a> Generator<'a> {
    pub fn new(name: &'a Ident, root: NodeId, err_id: NodeId, end_id: NodeId) -> Self {
        Generator {
            name,
            root,
            err_id,
            end_id,
            gotos: Map::default(),
        }
    }

    pub fn generate(&mut self, graph: &Graph<Token>) -> TokenStream {
        let mut out = TokenStream::new();

        for id in 0..graph.nodes().len() {
            if let Some(node) = graph.get(id) {
                out.append_all(self.generate_fn(id, node, graph));
            }
        }

        out
    }

    fn generate_fn(&mut self, id: NodeId, node: &Node<Token>, graph: &Graph<Token>) -> TokenStream {
        let body = match node {
            Node::Fork(fork) => self.generate_fork(fork, graph),
            Node::Rope(rope) => self.generate_rope(rope, graph),
            Node::Leaf(leaf) => self.generate_leaf(leaf),
        };
        let goto = self.generate_goto(id);

        quote! {
            fn #goto<'source, S: Source<'source>>(lex: &mut Lexer<S>) {
                #body
            }
        }
    }

    fn generate_fork(&mut self, fork: &Fork, graph: &Graph<Token>) -> TokenStream {
        quote! {

        }
    }

    fn generate_rope(&mut self, rope: &Rope, graph: &Graph<Token>) -> TokenStream {
        quote! {

        }
    }

    fn generate_leaf(&mut self, token: &Token) -> TokenStream {
        let name = self.name;
        let variant = &token.ident;
        quote! {
            lex.bump(1);
            lex.token = #name::#variant;
        }
    }

    fn generate_goto(&mut self, id: NodeId) -> &Ident {
        self.gotos.entry(id).or_insert_with(|| {
            Ident::new(&format!("goto_{}", id), Span::call_site())
        })
    }
}