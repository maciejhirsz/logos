use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Ident;
use fnv::FnvHashMap as Map;

use crate::graph::{Graph, Node, NodeId, Fork, Rope, Range};
use crate::leaf::Leaf;

type Bump = usize;

pub struct Generator<'a> {
    name: &'a Ident,
    root: NodeId,
    graph: &'a Graph<Leaf>,
    rendered: TokenStream,
    idents: Map<(NodeId, Context), Ident>,
    gotos: Map<(NodeId, Context), TokenStream>,
    stack: Vec<NodeId>,
}

impl<'a> Generator<'a> {
    pub fn new(name: &'a Ident, root: NodeId, graph: &'a Graph<Leaf>) -> Self {
        Generator {
            name,
            root,
            graph,
            rendered: TokenStream::new(),
            idents: Map::default(),
            gotos: Map::default(),
            stack: Vec::new(),
        }
    }

    pub fn generate(&mut self) -> &TokenStream {
        let root = self.goto(self.root, Context::new(0)).clone();

        assert_eq!(self.stack.len(), 0);

        self.rendered.append_all(root);
        &self.rendered
    }

    fn generate_fn(&mut self, id: NodeId, ctx: Context) -> TokenStream {
        self.stack.push(id);

        let body = match &self.graph[id] {
            Node::Fork(fork) => self.generate_fork(id, fork, ctx),
            Node::Rope(rope) => self.generate_rope(rope, ctx),
            Node::Leaf(leaf) => self.generate_leaf(leaf, ctx),
        };
        let ident = self.generate_ident(id, ctx);
        let out = quote! {
            #[inline]
            fn #ident<'s, S: Src<'s>>(lex: &mut Lexer<S>) {
                #body
            }
        };

        self.stack.pop();

        out
    }

    fn generate_fork(&mut self, this: NodeId, fork: &Fork, ctx: Context) -> TokenStream {
        let miss = match fork.miss {
            Some(id) => self.goto(id, Context::new(0)).clone(),
            None => quote! {
                lex.bump(1);
                lex.error()
            },
        };
        let end = if this == self.root {
            quote!(_end(lex))
        } else if fork.miss.is_some() {
            miss.clone()
        } else {
            quote!(lex.error())
        };

        let branches = fork.branches().map(|(range, id)| {
            let next = self.goto(id, Context::new(0));

            quote!(#range => {
                lex.bump(1);
                #next
            })
        });

        quote! {
            let byte = match lex.read() {
                Some(byte) => byte,
                None => return #end,
            };

            match byte {
                #(#branches)*
                _ => { #miss }
            }
        }
    }

    fn generate_rope(&mut self, rope: &Rope, ctx: Context) -> TokenStream {
        let miss = match rope.miss.first() {
            Some(id) => self.goto(id, Context::new(0)).clone(),
            None => quote!(lex.error()),
        };
        let len = rope.pattern.len();
        let then = self.goto(rope.then, Context::new(0));

        if let Some(bytes) = rope.pattern.to_bytes() {
            return quote! {
                match lex.read::<&[u8; #len]>() {
                    Some(&[#(#bytes),*]) => {
                        lex.bump(#len);
                        #then
                    },
                    _ => #miss,
                }
            };
        }

        let matches = rope.pattern.iter().enumerate().map(|(idx, range)| {
            quote! {
                match bytes[#idx] {
                    #range => (),
                    _ => return #miss,
                }
            }
        });

        quote! {
            match lex.read::<&[u8; #len]>() {
                Some(bytes) => {
                    #(#matches)*

                    lex.bump(#len);

                    #then
                },
                None => #miss,
            }
        }
    }

    fn generate_leaf(&mut self, leaf: &Leaf, ctx: Context) -> TokenStream {
        match leaf {
            Leaf::Trivia => {
                let root = self.goto(self.root, Context::new(0));

                quote! {
                    lex.trivia();
                    return #root;
                }
            },
            Leaf::Token { ident, .. } => {
                let name = self.name;

                quote! {
                    lex.token = #name::#ident;
                }
            },
        }
    }

    fn goto(&mut self, id: NodeId, ctx: Context) -> &TokenStream {
        if self.gotos.get(&(id, ctx)).is_none() {
            let ident = self.generate_ident(id, ctx);
            let call_site = quote!(#ident(lex));

            self.gotos.insert((id, ctx), call_site);

            let fun = self.generate_fn(id, ctx);

            self.rendered.append_all(fun);
        }
        &self.gotos[&(id, ctx)]
    }

    fn generate_ident(&mut self, id: NodeId, ctx: Context) -> &Ident {
        self.idents.entry((id, ctx)).or_insert_with(|| {
            let ident = format!("goto_{}_{}x{}", id, ctx.available, ctx.unbumped);

            Ident::new(&ident, Span::call_site())
        })
    }
}

/// This struct keeps track of bytes available to be read without
/// bounds checking across the tree.
///
/// For example, a branch that matches 4 bytes followed by a fork
/// with smallest branch containing of 2 bytes can do a bounds check
/// for 6 bytes ahead, and leave the remaining 2 byte array (fixed size)
/// to be handled by the fork, avoiding bound checks there.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Context {
    /// Amount of bytes available at local `arr` variable
    pub available: usize,

    /// Amount of bytes that haven't been bumped yet but should
    /// before a new read is performed
    pub unbumped: usize,
}

impl Context {
    pub const fn new(available: usize) -> Self {
        Context {
            available,
            unbumped: 0,
        }
    }

    pub const fn unbumped(unbumped: usize) -> Self {
        Context {
            available: 0,
            unbumped,
        }
    }

    pub const fn advance(self, n: usize) -> Self {
        Context {
            available: self.available - n,
            unbumped: self.unbumped + n,
        }
    }

    pub fn bump(self) -> TokenStream {
        match self.unbumped {
            0 => quote!(),
            n => quote!(lex.bump(#n);),
        }
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