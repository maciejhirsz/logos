use std::collections::BTreeMap as Map;

use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Ident;

use crate::graph::{Graph, Node, NodeId, Fork, Rope, Range};
use crate::leaf::Leaf;

pub struct Generator<'a> {
    name: &'a Ident,
    root: NodeId,
    gotos: Map<NodeId, Ident>,
}

impl<'a> Generator<'a> {
    pub fn new(name: &'a Ident, root: NodeId) -> Self {
        Generator {
            name,
            root,
            gotos: Map::default(),
        }
    }

    pub fn generate(&mut self, graph: &Graph<Leaf>) -> TokenStream {
        let mut out = TokenStream::new();

        for id in 0..graph.nodes().len() {
            if let Some(node) = graph.get(id) {
                out.append_all(self.generate_fn(id, node));
            }
        }

        let root = self.generate_goto(self.root);

        out.append_all(quote! {
            #root(lex)
        });

        out
    }

    fn generate_fn(&mut self, id: NodeId, node: &Node<Leaf>) -> TokenStream {
        let body = match node {
            Node::Fork(fork) => self.generate_fork(id, fork),
            Node::Rope(rope) => self.generate_rope(id, rope),
            Node::Leaf(leaf) => self.generate_leaf(leaf),
        };
        let goto = self.generate_goto(id);

        quote! {
            fn #goto<'s, S: Src<'s>>(lex: &mut Lexer<S>) {
                #body
            }
        }
    }

    fn generate_fork(&mut self, this: NodeId, fork: &Fork) -> TokenStream {
        let miss = match fork.miss {
            Some(id) => {
                let goto = self.generate_goto(id);
                quote!(#goto(lex))
            },
            // None if this == self.root => quote!(lex.error()),
            None => quote! {
                lex.bump(1);
                lex.error()
            },
        };

        let branches = fork.branches().map(|(range, id)| {
            let goto = self.generate_goto(id);

            quote!(#range => {
                lex.bump(1);
                #goto(lex)
            })
        });

        quote! {
            let byte = match lex.read() {
                Some(byte) => byte,
                None => return _end(lex),
            };

            match byte {
                #(#branches)*
                _ => { #miss }
            }
        }
    }

    fn generate_rope(&mut self, this: NodeId, rope: &Rope) -> TokenStream {
        let miss = match rope.miss.first() {
            Some(id) => {
                let goto = self.generate_goto(id);
                quote!(#goto(lex))
            },
            // None if this == self.root => quote!(_end),
            None => quote!(lex.error()),
        };

        let matches = rope.pattern.iter().map(|range| {
            quote! {
                match lex.read() {
                    Some(#range) => lex.bump(1),
                    Some(_) => return #miss,
                    None => return #miss,
                }
            }
        });

        let then = self.generate_goto(rope.then);

        quote! {
            #(#matches)*

            #then(lex)
        }
    }

    fn generate_leaf(&mut self, token: &Leaf) -> TokenStream {
        let name = self.name;
        let variant = &token.ident;
        quote! {
            lex.token = #name::#variant;
        }
    }

    fn generate_goto(&mut self, id: NodeId) -> &Ident {
        self.gotos.entry(id).or_insert_with(|| {
            Ident::new(&format!("goto_{}", id), Span::call_site())
        })
    }
}


macro_rules! match_quote {
    ($source:expr; $($byte:tt,)* ) => {match $source {
        $( $byte => quote!($byte), )*
        byte => quote!(#byte),
    }}
}

fn byte_to_tokens(byte: u8) -> TokenStream {
    match_quote! {
        byte;
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
        b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j',
        b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't',
        b'u', b'v', b'w', b'x', b'y', b'z',
        b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J',
        b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T',
        b'U', b'V', b'W', b'X', b'Y', b'Z',
        b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')',
        b'{', b'}', b'[', b']', b'<', b'>', b'-', b'=', b'_', b'+',
        b':', b';', b',', b'.', b'/', b'?', b'|', b'"', b'\'', b'\\',
    }
}

impl ToTokens for Range {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Range(start, end) = self;

        tokens.append_all(byte_to_tokens(*start));

        if start != end {
            tokens.append_all(quote!(..=));
            tokens.append_all(byte_to_tokens(*end));
        }
    }
}