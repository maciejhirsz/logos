use std::collections::hash_map::Entry;

use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Ident;
use fnv::{FnvHashMap as Map, FnvHashSet as Set};

use crate::graph::{Graph, Node, NodeId, Fork, Rope, Range};
use crate::leaf::Leaf;

pub struct Generator<'a> {
    name: &'a Ident,
    root: NodeId,
    graph: &'a Graph<Leaf>,
    rendered: TokenStream,
    fns: Set<(NodeId, Context)>,
    idents: Map<(NodeId, Context), Ident>,
    gotos: Map<(NodeId, Context), TokenStream>,
    meta: Map<NodeId, Meta>,
    stack: Vec<NodeId>,
}

#[derive(Debug)]
struct Meta {
    /// Number of references to this node
    refcount: usize,
    /// Ids of other nodes that point to this node while this
    /// node is on a stack (creating a loop)
    loop_entry_from: Vec<NodeId>,
}

impl Meta {
    fn new() -> Self {
        Meta {
            refcount: 0,
            loop_entry_from: Vec::new(),
        }
    }

    fn loop_entry(&mut self, id: NodeId) {
        if let Err(idx) = self.loop_entry_from.binary_search(&id) {
            self.loop_entry_from.insert(idx, id);
        }
    }
}

impl<'a> Generator<'a> {
    pub fn new(name: &'a Ident, root: NodeId, graph: &'a Graph<Leaf>) -> Self {
        Generator {
            name,
            root,
            graph,
            rendered: TokenStream::new(),
            fns: Set::default(),
            idents: Map::default(),
            gotos: Map::default(),
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
        let meta = self.meta.entry(this).or_insert_with(|| Meta::new());

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
            Node::Leaf(leaf) => (),
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
        let miss = ctx.miss(self);
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
        let miss = ctx.miss(self);
        let ranges = fork.branches().map(|(range, _)| range).collect::<Vec<_>>();

        let pat = quote!(#(#ranges)|*);
        let mut inner = quote !{
            if matches!(bytes[7], #pat) {
                lex.bump(8);
                continue;
            }
        };
        for i in (0..=6usize).rev() {
            inner = quote! {
                if matches!(bytes[#i], #pat) {
                    #inner
                    lex.bump(#i + 1);
                    return #miss;
                }
            };
        }

        quote! {
            // while let Some(bytes) = lex.read::<&[u8; 8]>() {
            //     #inner
            //     return #miss;
            // }

            // Go byte by byte if remaining source is too short
            while let Some(byte) = lex.read() {
                match byte {
                    #(#ranges)|* => lex.bump(1),
                    _ => break,
                }
            }

            #miss
        }
    }

    fn generate_rope(&mut self, rope: &Rope, ctx: Context) -> TokenStream {
        let miss = ctx.miss(self);
        let len = rope.pattern.len();
        let then = self.goto(rope.then, ctx.push(rope.pattern.len()));
        let read = match ctx.at {
            0 => quote!(lex.read::<&[u8; #len]>()),
            n => quote!(lex.read_at::<&[u8; #len]>(#n)),
        };

        if let Some(bytes) = rope.pattern.to_bytes() {
            return quote! {
                match #read {
                    Some(&[#(#bytes),*]) => #then,
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
            Leaf::Token { ident, .. } => {
                let name = self.name;

                quote! {
                    #bump
                    lex.token = #name::#ident;
                }
            },
        }
    }

    fn goto(&mut self, id: NodeId, mut ctx: Context) -> &TokenStream {
        let key = (id, ctx);

        if self.gotos.get(&key).is_none() {
            let mut bump = ctx.switch(self.graph[id].miss());

            if self.meta[&id].loop_entry_from.len() > 0 && ctx.at > 0 {
                bump = ctx.bump();
            }

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

    fn would_loop(&self, id: NodeId) -> Option<usize> {
        self.stack.iter().rev().position(|i| i == &id)
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
    /// Amount of bytes available at local `arr` variable
    // pub available: usize,

    /// Amount of bytes that haven't been bumped yet but should
    /// before a new read is performed
    pub at: usize,

    bumped: bool,

    miss: Option<NodeId>,
}

impl Context {
    pub const fn new() -> Self {
        Context {
            at: 0,
            bumped: false,
            miss: None,
        }
    }

    pub const fn goto_miss(self) -> Self {
        Context {
            at: 0,
            bumped: self.bumped,
            miss: None,
        }
    }

    pub fn switch(&mut self, miss: Option<NodeId>) -> Option<TokenStream> {
        self.miss = Some(miss?);
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

    pub fn miss(self, gen: &mut Generator) -> TokenStream {
        match self.miss {
            Some(id) => gen.goto(id, self.goto_miss()).clone(),
            None if self.bumped => quote!(lex.error()),
            None => quote!(_error(lex)),
        }
    }

    pub fn write_suffix(&self, buf: &mut String) {
        use std::fmt::Write;

        if self.at > 0 {
            let _ = write!(buf, "_at{}", self.at);
        }
        if let Some(id) = self.miss {
            let _ = write!(buf, "_else{}", id);
        }
        if self.bumped {
            buf.push_str("_x");
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