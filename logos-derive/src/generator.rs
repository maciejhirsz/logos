use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Ident;
use fnv::{FnvHashMap as Map, FnvHashSet as Set};

use crate::graph::{Graph, Node, NodeId, Fork, Rope, Range};
use crate::leaf::Leaf;

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
    meta: Map<NodeId, Meta>,
    /// Current execution stack, for keeping track of loops and such
    stack: Vec<NodeId>,
}

#[derive(Debug, Default)]
struct Meta {
    /// Number of references to this node
    refcount: usize,
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
        let rendered = quote! {
            macro_rules! _fast_loop {
                ($lex:ident, $test:ident, $miss:expr) => {
                    // Do one bounds check for multiple bytes till EOF
                    while let Some(arr) = $lex.read::<&[u8; 8]>() {
                        if $test(arr[0]) { if $test(arr[1]) { if $test(arr[2]) { if $test(arr[3]) {
                        if $test(arr[4]) { if $test(arr[5]) { if $test(arr[6]) { if $test(arr[7]) {

                        $lex.bump(8); continue;     } $lex.bump(7); return $miss; }
                        $lex.bump(6); return $miss; } $lex.bump(5); return $miss; }
                        $lex.bump(4); return $miss; } $lex.bump(3); return $miss; }
                        $lex.bump(2); return $miss; } $lex.bump(1); return $miss; }

                        return $miss;
                    }

                    while $lex.test($test) {
                        $lex.bump(1);
                    }

                    $miss
                };
            }
        };

        Generator {
            name,
            root,
            graph,
            rendered,
            fns: Set::default(),
            idents: Map::default(),
            gotos: Map::default(),
            tests: Map::default(),
            meta: Map::default(),
            stack: Vec::new(),
        }
    }

    pub fn generate(&mut self) -> &TokenStream {
        self.generate_meta(self.root, self.root);
        assert_eq!(self.stack.len(), 0);

        // panic!("{:#?}\n\n{:#?}", self.meta, self.graph);

        let root = self.goto(self.root, Context::new()).clone();

        assert_eq!(self.stack.len(), 0);

        self.rendered.append_all(root);
        &self.rendered
    }

    fn generate_meta(&mut self, this: NodeId, parent: NodeId) {
        let meta = self.meta.entry(this).or_default();

        meta.refcount += 1;

        if self.stack.contains(&this) {
            meta.loop_entry(parent);
        }
        if meta.refcount > 1 {
            return;
        }

        self.stack.push(this);

        match &self.graph[this] {
            Node::Fork(fork) => {
                for (_, id) in fork.branches() {
                    self.generate_meta(id, this);
                }
                if let Some(id) = fork.miss {
                    self.generate_meta(id, this);
                }
            },
            Node::Rope(rope) => {
                self.generate_meta(rope.then, this);
                if let Some(id) = rope.miss.first() {
                    self.generate_meta(id, this);
                }
            },
            Node::Leaf(_) => (),
        }

        self.stack.pop();
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
        let out = quote! {
            #[inline]
            fn #ident<'s, S: Src<'s>>(lex: &mut Lexer<S>) {
                #body
            }
        };

        self.stack.pop();
        self.rendered.append_all(out);
    }

    fn generate_fork(&mut self, this: NodeId, fork: &Fork, ctx: Context) -> TokenStream {
        if self.meta[&this].loop_entry_from.contains(&this) && fork.single_then().is_some() {
            return self.generate_fast_loop(fork, ctx);
        }
        let miss = ctx.miss(fork.miss, self);
        let end = if this == self.root {
            quote!(_end(lex))
        } else {
            miss.clone()
        };
        let read = match ctx.at {
            0 => quote!(lex.read()),
            n => quote!(lex.read_at(#n)),
        };
        let branches = fork.branches().map(|(range, id)| {
            let next = self.goto(id, ctx.push(1));

            quote!(#range => #next,)
        });

        quote! {
            let byte = match #read {
                Some(byte) => byte,
                None => return #end,
            };

            match byte {
                #(#branches)*
                _ => #miss,
            }
        }
    }

    fn generate_fast_loop(&mut self, fork: &Fork, ctx: Context) -> TokenStream {
        let miss = ctx.miss(fork.miss, self);
        let ranges = fork.branches().map(|(range, _)| range).collect::<Vec<_>>();
        let test = self.generate_test(ranges);

        quote! {
            _fast_loop!(lex, #test, #miss);
        }
    }

    fn generate_rope(&mut self, rope: &Rope, ctx: Context) -> TokenStream {
        let miss = ctx.miss(rope.miss.first(), self);
        let len = rope.pattern.len();
        let then = self.goto(rope.then, ctx.push(rope.pattern.len()));
        let read = match ctx.at {
            0 => quote!(lex.read::<&[u8; #len]>()),
            n => quote!(lex.read_at::<&[u8; #len]>(#n)),
        };

        if let Some(bytes) = rope.pattern.to_bytes() {
            let pat = byte_slice_literal(&bytes);

            return quote! {
                match #read {
                    Some(#pat) => #then,
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
            match #read {
                Some(bytes) => {
                    #(#matches)*

                    #then
                },
                None => #miss,
            }
        }
    }

    fn generate_leaf(&mut self, leaf: &Leaf, mut ctx: Context) -> TokenStream {
        let bump = ctx.bump();

        match leaf {
            Leaf::Trivia => {
                let root = self.goto(self.root, Context::new());

                quote! {
                    #bump
                    lex.trivia();
                    return #root;
                }
            },
            Leaf::Token { ident, callback, .. } => {
                let name = self.name;
                let out = quote! {
                    #bump
                    lex.token = #name::#ident;
                };

                match callback {
                    Some(callback) => quote! {
                        #out
                        #callback(lex);
                    },
                    None => out,
                }
            },
        }
    }

    fn goto(&mut self, id: NodeId, mut ctx: Context) -> &TokenStream {
        let key = (id, ctx);

        if !self.gotos.contains_key(&key) {
            let enters_loop = self.meta[&id].loop_entry_from.len() > 0;

            let bump = if enters_loop || !ctx.has_fallback() {
                ctx.switch(self.graph[id].miss())
            } else {
                None
            };

            let ident = self.generate_ident(id, ctx);
            let mut call_site = quote!(#ident(lex));

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

/// This struct keeps track of bytes available to be read without
/// bounds checking across the tree.
///
/// For example, a branch that matches 4 bytes followed by a fork
/// with smallest branch containing of 2 bytes can do a bounds check
/// for 6 bytes ahead, and leave the remaining 2 byte array (fixed size)
/// to be handled by the fork, avoiding bound checks there.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Context {
    /// Amount of bytes that haven't been bumped yet but should
    /// before a new read is performed
    pub at: usize,

    bumped: bool,

    fallback: Option<NodeId>,
}

impl Context {
    pub const fn new() -> Self {
        Context {
            at: 0,
            bumped: false,
            fallback: None,
        }
    }

    pub const fn fallback_ctx(self) -> Self {
        Context {
            at: 0,
            bumped: self.bumped,
            fallback: None,
        }
    }

    pub fn has_fallback(&self) -> bool {
        self.fallback.is_some()
    }

    pub fn switch(&mut self, miss: Option<NodeId>) -> Option<TokenStream> {
        if let Some(miss) = miss {
            self.fallback = Some(miss);
        }
        self.bump()
    }

    pub const fn push(self, n: usize) -> Self {
        Context {
            at: self.at + n,
            ..self
        }
    }

    pub fn bump(&mut self) -> Option<TokenStream> {
        match self.at {
            0 => None,
            n => {
                let tokens = quote!(lex.bump(#n););
                self.at = 0;
                self.bumped = true;
                Some(tokens)
            },
        }
    }

    pub fn miss(self, miss: Option<NodeId>, gen: &mut Generator) -> TokenStream {
        match (miss, self.fallback) {
            (Some(id), _) => gen.goto(id, self).clone(),
            (_, Some(id)) => gen.goto(id, self.fallback_ctx()).clone(),
            _ if self.bumped => quote!(lex.error()),
            _ => quote!(_error(lex)),
        }
    }

    pub fn write_suffix(&self, buf: &mut String) {
        use std::fmt::Write;

        if self.at > 0 {
            let _ = write!(buf, "_at{}", self.at);
        }
        if let Some(id) = self.fallback {
            let _ = write!(buf, "_ctx{}", id);
        }
        if self.bumped {
            buf.push_str("_x");
        }
    }
}

fn byte_slice_literal(bytes: &[u8]) -> TokenStream {
    if bytes.iter().any(|&b| b < 0x20 || b >= 0x7F) {
        return quote!(&[#(#bytes),*]);
    }

    let slice = std::str::from_utf8(bytes).unwrap();

    syn::parse_str(&format!("b{:?}", slice)).unwrap()
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