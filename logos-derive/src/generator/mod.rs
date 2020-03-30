use std::collections::BTreeMap;
use std::cmp::min;

use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Ident;
use fnv::{FnvHashMap as Map, FnvHashSet as Set};

use crate::graph::{Graph, Node, NodeId, Range};
use crate::leaf::Leaf;

mod fork;
mod leaf;
mod rope;
mod context;

pub use context::Context;

pub struct Generator<'a> {
    /// Name of the type we are implementing the `Logos` trait for
    name: &'a Ident,
    /// Id to the root node
    root: NodeId,
    /// Reference to the graph with all of the nodes
    graph: &'a Graph<Leaf>,
    /// Buffer with functions growing during generation
    rendered: TokenStream,
    /// Set of functions that have already been rendered
    fns: Set<(NodeId, Context)>,
    /// Function name identifiers
    idents: Map<(NodeId, Context), Ident>,
    /// Local function calls. Note: a call might change its context,
    /// so we can't use `idents` for this purpose.
    gotos: Map<(NodeId, Context), TokenStream>,
    /// Identifiers for helper functions matching a byte to a given
    /// set of ranges
    tests: Map<Vec<Range>, Ident>,
    /// Meta data collected for the nodes
    meta: BTreeMap<NodeId, Meta>,
    /// Current execution stack, for keeping track of loops and such
    stack: Vec<NodeId>,
}

#[derive(Debug, Default)]
struct Meta {
    /// Number of references to this node
    refcount: usize,
    /// Minimum number of bytes that ought to be read for this
    /// node to find a match
    min_read: usize,
    /// Marks whether or not this node leads to a loop entry node.
    is_loop_init: bool,
    /// Ids of other nodes that point to this node while this
    /// node is on a stack (creating a loop)
    loop_entry_from: Vec<NodeId>,
}

impl Meta {
    fn loop_entry(&mut self, id: NodeId) {
        if let Err(idx) = self.loop_entry_from.binary_search(&id) {
            self.loop_entry_from.insert(idx, id);
        }
    }
}

impl<'a> Generator<'a> {
    pub fn new(name: &'a Ident, root: NodeId, graph: &'a Graph<Leaf>) -> Self {
        let rendered = Self::fast_loop_macro();

        Generator {
            name,
            root,
            graph,
            rendered,
            fns: Set::default(),
            idents: Map::default(),
            gotos: Map::default(),
            tests: Map::default(),
            meta: BTreeMap::default(),
            stack: Vec::new(),
        }
    }

    pub fn generate(&mut self) -> &TokenStream {
        self.generate_meta(self.root, self.root);
        assert_eq!(self.stack.len(), 0);

        // panic!("{:#?}\n\n{:#?}", self.meta, self.graph);

        let root = self.goto(self.root, Context::default()).clone();

        assert_eq!(self.stack.len(), 0);

        self.rendered.append_all(root);
        &self.rendered
    }

    fn generate_meta(&mut self, this: NodeId, parent: NodeId) -> &Meta {
        let meta = self.meta.entry(this).or_default();
        let is_done = meta.refcount > 0;

        meta.refcount += 1;

        if self.stack.contains(&this) {
            meta.loop_entry(parent);
            self.meta.get_mut(&parent).unwrap().is_loop_init = true;
        }
        if is_done {
            return &self.meta[&this];
        }

        self.stack.push(this);

        let mut min_read;

        match &self.graph[this] {
            Node::Fork(fork) => {
                min_read = usize::max_value();
                for (_, id) in fork.branches() {
                    let meta = self.generate_meta(id, this);

                    if meta.is_loop_init {
                        min_read = 1;
                    } else {
                        min_read = min(min_read, meta.min_read + 1);
                    }
                }
                if let Some(id) = fork.miss {
                    self.generate_meta(id, this);
                }
                if min_read == usize::max_value() {
                    min_read = 0;
                }
            },
            Node::Rope(rope) => {
                min_read = rope.pattern.len();
                let meta = self.generate_meta(rope.then, this);

                if !meta.is_loop_init {
                    min_read += meta.min_read;
                }

                if let Some(id) = rope.miss.first() {
                    self.generate_meta(id, this);
                }
            },
            Node::Leaf(_) => min_read = 0,
        }

        self.meta.get_mut(&this).unwrap().min_read = min_read;
        self.stack.pop();

        &self.meta[&this]
    }

    fn generate_fn(&mut self, id: NodeId, ctx: Context) {
        if self.fns.contains(&(id, ctx)) {
            return;
        }
        self.fns.insert((id, ctx));

        self.stack.push(id);

        let body = match &self.graph[id] {
            Node::Fork(fork) => self.generate_fork(id, fork, ctx),
            Node::Rope(rope) => self.generate_rope(rope, ctx),
            Node::Leaf(leaf) => self.generate_leaf(leaf, ctx),
        };
        let ident = self.generate_ident(id, ctx);
        let props = ctx.fn_props();
        let out = quote! {
            #[inline]
            fn #ident<'s, S: Src<'s>>(lex: &mut Lexer<S> #props) {
                #body
            }
        };

        self.stack.pop();
        self.rendered.append_all(out);
    }

    fn goto(&mut self, id: NodeId, mut ctx: Context) -> &TokenStream {
        let key = (id, ctx);

        if !self.gotos.contains_key(&key) {
            let meta = &self.meta[&id];
            let enters_loop = meta.loop_entry_from.len() > 0;

            let bump = if enters_loop || !ctx.can_backtrack() {
                ctx.switch(self.graph[id].miss())
            } else {
                None
            };
            if meta.min_read == 0 || ctx.remainder() < meta.min_read  {
                ctx.wipe();
            }

            let ident = self.generate_ident(id, ctx);
            let args = ctx.call_args();
            let mut call_site = quote!(#ident(lex #args));

            if let Some(bump) = bump {
                call_site = quote!({
                    #bump
                    #call_site
                });
            }
            self.gotos.insert(key, call_site);
            self.generate_fn(id, ctx);
        }
        &self.gotos[&key]
    }

    fn generate_ident(&mut self, id: NodeId, ctx: Context) -> &Ident {
        self.idents.entry((id, ctx)).or_insert_with(|| {
            let mut ident = format!("goto{}", id);

            ctx.write_suffix(&mut ident);

            Ident::new(&ident, Span::call_site())
        })
    }

    /// Returns an identifier to a function that matches a byte to any
    /// of the provided ranges. This will generate either a simple
    /// match expression, or use a lookup table internally.
    fn generate_test(&mut self, ranges: Vec<Range>) -> &Ident {
        if !self.tests.contains_key(&ranges) {
            let idx = self.tests.len();
            let ident = Ident::new(&format!("pattern{}", idx), Span::call_site());

            let body = match ranges.len() {
                0..=2 => {
                    quote! {
                        match byte {
                            #(#ranges)|* => true,
                            _ => false,
                        }
                    }
                },
                _ => {
                    let mut table = [false; 256];

                    for byte in ranges.iter().flat_map(|range| *range) {
                        table[byte as usize] = true;
                    }
                    let ltrue = quote!(TT);
                    let lfalse = quote!(__);
                    let table = table.iter().map(|x| if *x { &ltrue } else { &lfalse });

                    quote! {
                        const #ltrue: bool = true;
                        const #lfalse: bool = false;

                        static LUT: [bool; 256] = [#( #table ),*];

                        LUT[byte as usize]
                    }
                }
            };
            self.rendered.append_all(quote! {
                #[inline]
                fn #ident(byte: u8) -> bool {
                    #body
                }
            });
            self.tests.insert(ranges.clone(), ident);
        }
        &self.tests[&ranges]
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