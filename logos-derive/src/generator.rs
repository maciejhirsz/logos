use std::marker::PhantomData;
use std::rc::Rc;
use syn::{Ident, parse_str};
use quote::{quote, ToTokens};
use proc_macro2::{TokenStream, Span};
use rustc_hash::FxHashMap as HashMap;

use crate::handlers::{Tree, Fallback};
use crate::tree::{Node, Branch, Fork, ForkKind, Leaf};
use crate::regex::{Regex, Pattern};

pub struct Generator<'a> {
    enum_name: &'a Ident,
    fns: TokenStream,
    fns_constructed: HashMap<usize, Ident>,
    patterns: HashMap<Pattern, Ident>,
    callbacks: HashMap<&'a Ident, Ident>,
}

/// Get a pointer to the Rc as `usize`. We can use this
/// as the key in the HashMap for constructed handlers, which
/// is way faster to hash than the entire tree. It also means
/// we don't have to derive `Hash` everywhere.
fn get_rc_ptr<T>(rc: &Rc<T>) -> usize {
    Rc::into_raw(rc.clone()) as usize
}

/// This struct keeps track of bytes available to be read without
/// bounds checking across the tree.
///
/// For example, a branch that matches 4 bytes followed by a fork
/// with smallest branch containing of 2 bytes can do a bounds check
/// for 6 bytes ahead, and leave the remaining 2 byte array (fixed size)
/// to be handled by the fork, avoiding bound checks there.
#[derive(Clone, Copy, Default)]
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

pub enum MatchDefault {
    Repeat(TokenStream),
    Once(TokenStream),
    None,
}

impl<'a> Generator<'a> {
    pub fn new(enum_name: &'a Ident) -> Self {
        Generator {
            enum_name,
            fns: TokenStream::new(),
            fns_constructed: HashMap::default(),
            patterns: HashMap::default(),
            callbacks: HashMap::default(),
        }
    }

    pub fn set_callback(&mut self, variant: &'a Ident, func: Ident) {
        self.callbacks.insert(variant, func);
    }

    pub fn print_tree(&mut self, tree: Rc<Tree<'a>>) -> TokenStream {
        let ptr = get_rc_ptr(&tree);

        if !self.fns_constructed.contains_key(&ptr) {
            let mut tokens = Vec::new();
            tree.node.get_tokens(&mut tokens);
            let body = self.tree_to_fn_body(tree.clone());

            let handler = Some(format!("_handle_{}", self.fns_constructed.len()))
                            .into_iter()
                            .chain(
                                tokens.into_iter()
                                      .take(3)
                                      .map(|token| format!("{}", token).to_lowercase())
                            )
                            .collect::<Vec<_>>()
                            .join("_");

            let handler = Ident::new(&handler, Span::call_site());

            self.fns.extend(quote! {
                #[allow(unreachable_code)]
                fn #handler<'source, S: ::logos::Source<'source>>(lex: &mut Lexer<S>) {
                    lex.bump(1);
                    #body
                }
            });

            self.fns_constructed.insert(ptr, handler);
        }

        let handler = self.fns_constructed.get(&ptr).unwrap();

        quote!(Some(#handler))
    }

    fn tree_to_fn_body(&mut self, mut tree: Rc<Tree<'a>>) -> TokenStream {
        // At this point all Rc pointers should be unique
        let tree = Rc::make_mut(&mut tree);

        if let Some(Fallback { boundary, leaf }) = &tree.fallback {
            let regex = Regex::from(boundary.clone());
            let mut then = Some(Node::Leaf(leaf.clone()).boxed());

            let fallback = LooseGenerator(self).print_simple_repeat(&regex, &mut then, Context::new(0));

            FallbackGenerator {
                gen: self,
                fallback,
                boundary: boundary.clone(),
            }.print(&mut tree.node)
        } else {
            LooseGenerator(self).print(&mut tree.node)
        }
    }

    fn pattern_to_fn(&mut self, pattern: &Pattern) -> TokenStream {
        let idx = self.patterns.len();

        let patterns = &mut self.patterns;
        let fns = &mut self.fns;

        let function = patterns.entry(pattern.clone()).or_insert_with(|| {
            let chars = format!("{:?}", pattern)
                            .bytes()
                            .filter(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                            .map(|b| b as char)
                            .collect::<String>();
            let function = Ident::new(&format!("_pattern_{}_{}", idx, chars), Span::call_site());

            let tokens = match pattern.weight() {
                1 => {
                    quote! {
                        #[inline]
                        fn #function(byte: u8) -> bool {
                            byte == #pattern
                        }
                    }
                },
                2 => {
                    quote! {
                        #[inline]
                        fn #function(byte: u8) -> bool {
                            match byte {
                                #pattern => true,
                                _ => false,
                            }
                        }
                    }
                },
                _ => {
                    let bytes: Vec<u8> = pattern.to_bytes();

                    let mut table = [false; 256];

                    for byte in bytes {
                        table[byte as usize] = true;
                    }

                    let ltrue = quote!(TT);
                    let lfalse = quote!(__);

                    let table = table.iter().map(|x| if *x { &ltrue } else { &lfalse });

                    quote! {
                        #[inline]
                        fn #function(byte: u8) -> bool {
                            const #ltrue: bool = true;
                            const #lfalse: bool = false;

                            static LUT: [bool; 256] = [#( #table ),*];

                            LUT[byte as usize]
                        }
                    }
                }
            };

            fns.extend(tokens);

            function
        });

        quote!(#function)
    }

    pub fn fns(self) -> TokenStream {
        self.fns
    }
}

pub trait SubGenerator<'a>: Sized {
    fn gen(&mut self) -> &mut Generator<'a>;

    fn print(&mut self, node: &mut Node) -> TokenStream;

    fn print_leaf(&mut self, leaf: &Leaf, ctx: Context) -> TokenStream;

    fn print_then(&mut self, then: &mut Option<Box<Node>>, ctx: Context) -> TokenStream {
        if let Some(node) = then {
            self.print_node(&mut **node, ctx)
        } else {
            ctx.bump()
        }
    }

    fn print_then_plain(&mut self, then: &mut Option<Box<Node>>, ctx: Context) -> TokenStream {
        self.print_then(then, ctx)
    }

    fn print_lex_read(&mut self, code: TokenStream, default: MatchDefault, bytes: usize, ctx: Context) -> TokenStream {
        let bump = ctx.bump();

        let (cond, default) = match default {
            MatchDefault::Repeat(default) => (quote!(while), default),
            MatchDefault::Once(default)   => (quote!(if), default),
            MatchDefault::None            => (quote!(if), TokenStream::new()),
        };

        quote! {
            #bump

            #cond let Some(arr) = lex.read::<&[u8; #bytes]>() {
                #code
            }

            #default
        }
    }

    fn print_branch(&mut self, branch: &mut Branch, ctx: Context) -> TokenStream {
        if branch.regex.len() == 0 {
            return self.print_then(&mut branch.then, ctx);
        }

        let len = branch.regex.len();

        if ctx.available < len {
            let read = branch.min_bytes();

            if read == len || len > 16 {
                let test = self.regex_to_test(branch.regex.patterns(), Context::new(0));
                let next = self.print_then(&mut branch.then, Context::unbumped(len));
                let bump = ctx.bump();

                return quote! {
                    #bump

                    if #test {
                        #next
                    }
                };
            }

            let branch = self.print_branch(branch, Context::new(read));

            return self.print_lex_read(branch, MatchDefault::None, read, ctx);
        }

        let (source, split) = if ctx.available > len {
            (quote!(chunk), quote!(let (chunk, arr): (&[u8; #len], _) = arr.split();))
        } else {
            (quote!(arr), TokenStream::new())
        };

        let test = self.chunk_to_test(source, branch.regex.patterns());
        let next = self.print_then(&mut branch.then, ctx.advance(len));

        quote! {
            #split

            if #test {
                #next
            }
        }
    }

    fn print_branch_maybe(&mut self, branch: &mut Branch, ctx: Context) -> TokenStream {
        MaybeGenerator(self, PhantomData).print_branch(branch, ctx)
    }

    fn print_branch_loop(&mut self, branch: &mut Branch, ctx: Context) -> TokenStream {
        LoopGenerator(self, PhantomData).print_branch(branch, ctx)
    }

    fn print_fork(&mut self, fork: &mut Fork, ctx: Context) -> TokenStream {
        if fork.arms.len() == 0 {
            return self.print_then(&mut fork.then, ctx);
        }

        if fork.arms.len() == 1 {
            let arm = &mut fork.arms[0];
            let regex = &arm.regex;

            match fork.kind {
                ForkKind::Plain => {},
                ForkKind::Maybe => {
                    let then = &mut arm.then;
                    let otherwise = &mut fork.then;

                    return self.print_simple_maybe(regex, then, otherwise, ctx);
                },
                ForkKind::Repeat => {
                    if arm.then.is_none() {
                        let then = &mut fork.then;

                        // FIXME: Pass on ctx
                        return self.print_simple_repeat(regex, then, ctx);
                    }
                }
            }
        }

        self.print_fork_as_match(fork, ctx)
    }

    fn print_fork_as_match(&mut self, fork: &mut Fork, ctx: Context) -> TokenStream {
        let inside_a_loop = fork.kind == ForkKind::Repeat
            || (fork.kind == ForkKind::Maybe && fork.arms.iter().any(|arm| arm.then.is_none()));

        if ctx.available < 1 {
            // FIXME: Cap `read` to 16
            let read = fork.min_bytes();

            let default = match fork.then.take() {
                None => MatchDefault::None,
                Some(then) => match inside_a_loop {
                    false => MatchDefault::Once(self.print_then(&mut Some(then), Context::new(0))),
                    true => MatchDefault::Repeat(self.print_then(&mut Some(then), Context::new(0))),
                },
            };

            let fork = self.print_fork_as_match(fork, Context::new(read));

            return self.print_lex_read(fork, default, read, ctx);
        }

        let kind = fork.kind;

        let branches = fork.arms.iter_mut().map(|branch| {
            let test = {
                let pattern = branch.regex.unshift();

                if pattern.weight() <= 2 {
                    quote!(#pattern)
                } else {
                    let test = self.gen().pattern_to_fn(&pattern);

                    quote!(byte if #test(byte))
                }
            };

            let branch = match kind {
                ForkKind::Plain  => self.print_branch(branch, ctx.advance(1)),
                ForkKind::Maybe  => self.print_branch_maybe(branch, ctx.advance(1)),
                ForkKind::Repeat => self.print_branch_loop(branch, ctx.advance(1)),
            };

            quote! {
                #test => {
                    #branch
                },
            }
        }).collect::<TokenStream>();

        let (source, split) = if ctx.available > 1 {
            (quote!(byte), quote!(let (byte, arr): (u8, _) = arr.split();))
        } else {
            (quote!(arr[0]), TokenStream::new())
        };

        if !inside_a_loop {
            let default = match kind {
                ForkKind::Plain => self.print_then_plain(&mut fork.then, ctx),
                _               => self.print_then(&mut fork.then, ctx),
            };

            quote! {
                #split

                match #source {
                    #branches
                    _ => {
                        #default
                    },
                }
            }
        } else {
            // FIXME: bump before fallback?
            let fallback = self.print_fallback();

            quote! {
                #split

                match #source {
                    #branches
                    _ => break,
                }

                #fallback
            }
        }
    }

    fn print_simple_repeat(&mut self, regex: &Regex, then: &mut Option<Box<Node>>, ctx: Context) -> TokenStream {
        match regex.len() {
            0 => self.print_then(then, ctx),
            1 => {
                let bump = ctx.bump();
                let next = self.print_then(then, Context::default());
                let fun = self.gen().pattern_to_fn(regex.first());

                quote! {
                    #bump

                    loop {
                        if let Some(arr) = lex.read::<&[u8; 16]>() {
                            // There is at least 16 bytes left until EOF, so we get a pointer to a fixed size array.
                            //
                            // All those reads are now virtually free, all of the calls should be inlined.
                            if #fun(arr[0])  { if #fun(arr[1])  { if #fun(arr[2])  { if #fun(arr[3])  {
                            if #fun(arr[4])  { if #fun(arr[5])  { if #fun(arr[6])  { if #fun(arr[7])  {
                            if #fun(arr[8])  { if #fun(arr[9])  { if #fun(arr[10]) { if #fun(arr[11]) {
                            if #fun(arr[12]) { if #fun(arr[13]) { if #fun(arr[14]) { if #fun(arr[15]) {

                            // Continue the loop if all 16 bytes are matching, else break at appropriate branching
                            lex.bump(16); continue; } lex.bump(15); break; } lex.bump(14); break; } lex.bump(13); break; }
                            lex.bump(12); break;    } lex.bump(11); break; } lex.bump(10); break; } lex.bump(9);  break; }
                            lex.bump(8);  break;    } lex.bump(7);  break; } lex.bump(6);  break; } lex.bump(5);  break; }
                            lex.bump(4);  break;    } lex.bump(3);  break; } lex.bump(2);  break; } lex.bump(1);  break; }

                            break;
                        } else {
                            // There weren't enough bytes for the fast path.
                            // handle the remainder by looping one byte at a time.
                            while lex.read().map(#fun).unwrap_or(false) {
                                lex.bump(1);
                            }

                            break;
                        }
                    }

                    #next
                }
            },
            _ => {
                let bump = ctx.bump();
                let next = self.print_then(then, Context::new(0));
                let test = self.regex_to_test(regex.patterns(), Context::new(0));
                let len = regex.len();

                quote! {
                    #bump

                    while #test {
                        lex.bump(#len);
                    }

                    #next
                }
            }
        }
    }

    fn print_simple_maybe(&mut self, regex: &Regex, then: &mut Option<Box<Node>>, otherwise: &mut Option<Box<Node>>, ctx: Context) -> TokenStream {
        let len = regex.len();
        let bump = ctx.bump();
        let test = self.regex_to_test(regex.patterns(), Context::new(0));

        match then.is_some() {
            true => {
                let otherwise = self.print_then(otherwise, Context::new(0));
                let then = self.print_then(then, Context::unbumped(len));

                quote! {
                    #bump

                    if #test {
                        #then
                    }

                    #otherwise
                }
            },
            false => {
                let next = self.print_then(otherwise, Context::new(0));
                let test = self.regex_to_test(regex.patterns(), Context::new(0));

                quote! {
                    #bump

                    if #test {
                        lex.bump(#len);
                    }

                    #next
                }
            },
        }
    }

    /// Convert a slice of `Pattern`s into a test that can be inserted into
    /// an `if ____ {` or `while ____ {`.
    fn regex_to_test(&mut self, patterns: &[Pattern], ctx: Context) -> TokenStream {
        let read = match ctx.unbumped {
            0 => quote!(lex.read()),
            n => quote!(lex.lookahead(#n)),
        };

        // Fast path optimization for bytes
        if patterns.iter().all(Pattern::is_byte) {
            match patterns.len() {
                1 => {
                    let pattern = &patterns[0];

                    return quote!(#read == Some(#pattern));
                },
                2...16 => {
                    let literal = byte_literal(patterns);

                    return quote!(#read == Some(#literal));
                },
                _ => {}
            }
        }

        // Fast path optimization for single pattern
        if patterns.len() == 1 {
            let pattern = &patterns[0];
            let fun = self.gen().pattern_to_fn(pattern);

            return quote!(#read.map(#fun).unwrap_or(false));
        }

        let test = patterns.chunks(16).enumerate().map(|(idx, chunk)| {
            let offset = 16 * idx + ctx.unbumped;
            let len = chunk.len();

            let source = match offset {
                0 => quote!(lex.read::<&[u8; #len]>()),
                _ => quote!(lex.lookahead::<&[u8; #len]>(#offset)),
            };

            let test = self.chunk_to_test(quote!(chunk), chunk);

            quote!(#source.map(|chunk| #test).unwrap_or(false))
        });

        quote!(#(#test)&&*)
    }

    /// Convert a chunk of up to 16 `Pattern`s into a test
    fn chunk_to_test(&mut self, source: TokenStream, chunk: &[Pattern]) -> TokenStream {
        let source = &source;

        if chunk.iter().all(Pattern::is_byte) {
            let literal = byte_literal(chunk);

            quote!(#source == #literal)
        } else {
            let chunk = chunk.iter().enumerate().map(|(idx, pat)| {
                self.pattern_to_test(quote!(#source[#idx]), pat)
            });

            quote!(#(#chunk)&&*)
        }
    }

    /// Convert an individual `Pattern` into a test
    fn pattern_to_test(&mut self, source: TokenStream, pattern: &Pattern) -> TokenStream {
        if pattern.is_byte() {
            quote!(#source == #pattern)
        } else {
            let fun = self.gen().pattern_to_fn(pattern);

            quote!(#fun(#source))
        }
    }

    fn print_node(&mut self, node: &mut Node, ctx: Context) -> TokenStream {
        match node {
            Node::Leaf(leaf) => self.print_leaf(leaf, ctx),
            Node::Branch(branch) => self.print_branch(branch, ctx),
            Node::Fork(fork) => self.print_fork(fork, ctx),
        }
    }

    fn print_fallback(&mut self) -> TokenStream;
}

pub struct LooseGenerator<'a: 'b, 'b>(&'b mut Generator<'a>);
pub struct FallbackGenerator<'a: 'b, 'b> {
    gen: &'b mut Generator<'a>,
    boundary: Pattern,
    fallback: TokenStream,
}

pub struct LoopGenerator<'a: 'b, 'b: 'c, 'c, SubGen>(&'c mut SubGen, PhantomData<LooseGenerator<'a, 'b>>)
where
    SubGen: SubGenerator<'a> + 'c;

pub struct MaybeGenerator<'a: 'b, 'b: 'c, 'c, SubGen>(&'c mut SubGen, PhantomData<LooseGenerator<'a, 'b>>)
where
    SubGen: SubGenerator<'a> + 'c;

impl<'a, 'b> SubGenerator<'a> for LooseGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.0
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        let body = self.print_node(node, Context::new(0));

        quote! {
            #body

            lex.token = ::logos::Logos::ERROR;
        }
    }

    fn print_leaf(&mut self, leaf: &Leaf, ctx: Context) -> TokenStream {
        let name = self.gen().enum_name;
        let bump = ctx.bump();

        let variant = leaf.token;
        let callback = leaf.callback.as_ref().or_else(|| self.gen().callbacks.get(variant));

        match callback {
            Some(callback) => quote! {
                #bump
                lex.token = #name::#variant;
                return #callback(lex);
            },
            None => quote! {
                #bump
                return lex.token = #name::#variant;
            },
        }
    }

    fn print_fallback(&mut self) -> TokenStream {
        quote!(return lex.token = ::logos::Logos::ERROR;)
    }
}

impl<'a, 'b> SubGenerator<'a> for FallbackGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.gen
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        let body = self.print_node(node, Context::new(0));
        let fallback = &self.fallback;

        quote! {
            #body

            #fallback
        }
    }

    fn print_leaf(&mut self, leaf: &Leaf, ctx: Context) -> TokenStream {
        let name = self.gen().enum_name;
        let pattern_fn = self.gen.pattern_to_fn(&self.boundary);
        let bump = ctx.bump();

        let variant = leaf.token;
        let callback = leaf.callback.as_ref().or_else(|| self.gen().callbacks.get(variant));

        match callback {
            Some(callback) => {
                quote! {
                    #bump
                    if !lex.read().map(#pattern_fn).unwrap_or(false) {
                        lex.token = #name::#variant;
                        return #callback(lex);
                    }
                }
            },
            None => {
                quote! {
                    #bump
                    if !lex.read().map(#pattern_fn).unwrap_or(false) {
                        return lex.token = #name::#variant;
                    }
                }
            }
        }
    }

    fn print_fallback(&mut self) -> TokenStream {
        self.fallback.clone()
    }
}

impl<'a, 'b, 'c, SubGen> SubGenerator<'a> for LoopGenerator<'a, 'b, 'c, SubGen>
where
    SubGen: SubGenerator<'a>
{
    fn gen(&mut self) -> &mut Generator<'a> {
        self.0.gen()
    }

    fn print_branch_maybe(&mut self, branch: &mut Branch, ctx: Context) -> TokenStream {
        MaybeGenerator(self.0, PhantomData).print_branch(branch, ctx)
    }

    fn print_branch_loop(&mut self, branch: &mut Branch, ctx: Context) -> TokenStream {
        self.print_branch(branch, ctx)
    }

    fn print_then(&mut self, then: &mut Option<Box<Node>>, ctx: Context) -> TokenStream {
        if let Some(node) = then {
            self.print_node(&mut **node, ctx)
        } else {
            let bump = ctx.bump();

            quote! {
                #bump
                continue;
            }
        }
    }

    fn print_then_plain(&mut self, then: &mut Option<Box<Node>>, ctx: Context) -> TokenStream {
        self.0.print_then(then, ctx)
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        self.0.print(node)
    }

    fn print_leaf(&mut self, leaf: &Leaf, ctx: Context) -> TokenStream {
        self.0.print_leaf(leaf, ctx)
    }

    fn print_fallback(&mut self) -> TokenStream {
        self.0.print_fallback()
    }
}

impl<'a, 'b, 'c, SubGen> SubGenerator<'a> for MaybeGenerator<'a, 'b, 'c, SubGen>
where
    SubGen: SubGenerator<'a>
{
    fn gen(&mut self) -> &mut Generator<'a> {
        self.0.gen()
    }

    fn print_branch_maybe(&mut self, branch: &mut Branch, ctx: Context) -> TokenStream {
        self.print_branch(branch, ctx)
    }

    fn print_branch_loop(&mut self, branch: &mut Branch, ctx: Context) -> TokenStream {
        LoopGenerator(self.0, PhantomData).print_branch(branch, ctx)
    }

    fn print_then(&mut self, then: &mut Option<Box<Node>>, ctx: Context) -> TokenStream {
        if let Some(node) = then {
            self.0.print_node(&mut **node, ctx)
        } else {
            quote!(break;)
        }
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        self.0.print(node)
    }

    fn print_leaf(&mut self, leaf: &Leaf, ctx: Context) -> TokenStream {
        self.0.print_leaf(leaf, ctx)
    }

    fn print_fallback(&mut self) -> TokenStream {
        self.0.print_fallback()
    }
}

macro_rules! match_quote {
    ($source:expr; $($byte:tt,)* ) => {match $source {
        $( $byte => quote!($byte), )*
        byte => quote!(#byte),
    }}
}

fn byte_literal(patterns: &[Pattern]) -> TokenStream {
    assert!(
        patterns.iter().all(Pattern::is_byte),
        "Internal Error: Trying to create a byte literal from non-byte patterns"
    );

    let chars: String = patterns.iter().filter_map(|pat| {
        match pat {
            Pattern::Byte(byte) if *byte >= 0x20 && *byte < 0x80 => {
                Some(*byte as char)
            },
            _ => None
        }
    }).collect();

    if chars.len() == patterns.len() {
        let literal = format!("b{:?}", chars);

        return parse_str(&literal).unwrap();
    }

    quote!(&[#(#patterns),*])
}

impl ToTokens for Pattern {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            // This is annoying, but it seems really hard to make quote!
            // print byte chars instead of integers otherwise
            Pattern::Byte(byte) => tokens.extend(match_quote! {
                *byte;
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
            }),
            Pattern::Range(from, to) => tokens.extend(quote!(#from..=#to)),
            Pattern::Class(ref class) => tokens.extend(quote!(#( #class )|*)),
        }
    }
}
