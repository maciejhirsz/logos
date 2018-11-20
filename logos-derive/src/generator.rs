use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use syn::Ident;
use quote::{quote, ToTokens};
use proc_macro2::{TokenStream, Span};

use tree::{Node, Branch, ForkKind};
use regex::{Regex, Pattern};

pub struct Generator<'a> {
    enum_name: &'a Ident,
    fns: TokenStream,
    fns_constructed: HashMap<usize, Ident>,
    patterns: HashMap<Pattern, Ident>,
}

/// Get a pointer to the Rc as `usize`. We can use this
/// as the key in the HashMap for constructed handlers, which
/// is way faster to hash than the entire tree. It also means
/// we don't have to derive `Hash` everywhere.
fn get_rc_ptr<T>(rc: &Rc<T>) -> usize {
    Rc::into_raw(rc.clone()) as usize
}

impl<'a> Generator<'a> {
    pub fn new(enum_name: &'a Ident) -> Self {
        Generator {
            enum_name,
            fns: TokenStream::new(),
            fns_constructed: HashMap::new(),
            patterns: HashMap::new(),
        }
    }

    pub fn print_tree(&mut self, tree: Rc<Node<'a>>) -> TokenStream {
        let ptr = get_rc_ptr(&tree);

        if !self.fns_constructed.contains_key(&ptr) {
            let mut tokens = Vec::new();
            tree.get_tokens(&mut tokens);
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
                fn #handler<S: ::logos::Source>(lex: &mut Lexer<S>) {
                    lex.bump();
                    #body
                }
            });

            self.fns_constructed.insert(ptr, handler);
        }

        let handler = self.fns_constructed.get(&ptr).unwrap();

        quote!(Some(#handler))
    }

    fn tree_to_fn_body(&mut self, mut tree: Rc<Node<'a>>) -> TokenStream {
        // At this point all Rc pointers should be unique
        let tree = Rc::make_mut(&mut tree);

        if tree.exhaustive() {
            ExhaustiveGenerator(self).print(tree)
        } else {
            if let Some(mut fallback) = tree.fallback() {
                let boundary = fallback.regex.first().clone();
                let fallback = LooseGenerator(self).print_then(&mut fallback.then);

                FallbackGenerator {
                    gen: self,
                    fallback,
                    boundary,
                }.print(tree)
            } else {
                LooseGenerator(self).print(tree)
            }
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

    fn print_token(&mut self, variant: &Ident) -> TokenStream;

    fn print_then(&mut self, then: &mut Option<Box<Node>>) -> TokenStream {
        if let Some(node) = then {
            self.print_node(&mut **node)
        } else {
            quote!( {} )
        }
    }

    fn print_branch(&mut self, branch: &mut Branch) -> TokenStream {
        if branch.regex.len() == 0 {
            return self.print_then(&mut branch.then);
        }

        let (first, rest) = self.regex_to_test(branch.regex.consume());
        let next = self.print_branch(branch);

        quote! {
            if #first #(&& #rest)* {
                lex.bump();

                #next
            }
        }
    }

    fn print_simple_repeat(&mut self, regex: &Regex, then: &mut Option<Box<Node>>) -> TokenStream {
        if regex.len() == 0 {
            return self.print_then(then);
        }

        let (first, rest) = self.regex_to_test(regex.patterns());
        let next = self.print_then(then);

        quote!({
            while #first #(&& #rest)* {
                lex.bump();
            }

            #next
        })
    }

    fn print_simple_maybe(&mut self, regex: &Regex, then: &mut Option<Box<Node>>) -> TokenStream {
        if regex.len() == 0 {
            return self.print_then(then);
        }

        let (first, rest) = self.regex_to_test(regex.patterns());
        let next = self.print_then(then);

        quote!({
            if #first #(&& #rest)* {
                lex.bump();
            }

            #next
        })
    }

    fn regex_to_test(&mut self, patterns: &[Pattern]) -> (TokenStream, Vec<TokenStream>) {
        let first = &patterns[0];
        let rest = &patterns[1..];

        let first = if first.is_byte() {
            quote!(lex.read() == #first)
        } else {
            let function = self.gen().pattern_to_fn(first);

            quote!(#function(lex.read()))
        };

        let rest = rest.iter().map(|pat| {
            if pat.is_byte() {
                quote!(lex.next() == #pat)
            } else {
                let function = self.gen().pattern_to_fn(pat);

                quote!(#function(lex.next()))
            }
        });

        (quote!(#first), rest.collect())
    }

    fn print_node(&mut self, node: &mut Node) -> TokenStream {
        match node {
            Node::Token(token) => self.print_token(token),
            Node::Branch(branch) => self.print_branch(branch),
            Node::Fork(fork) => {
                if fork.arms.len() == 0 {
                    return self.print_then(&mut fork.then);
                }

                // if fork.kind != ForkKind::Plain
                //     && fork.arms.len() == 1
                //     && fork.arms[0].then.is_none()
                // {
                //     let regex = &fork.arms[0].regex;
                //     let then = &mut fork.then;

                //     return if fork.kind == ForkKind::Repeat {
                //         self.print_simple_repeat(regex, then)
                //     } else {
                //         self.print_simple_maybe(regex, then)
                //     };
                // }

                let kind = fork.kind;

                let branches = fork.arms.iter_mut().map(|branch| {
                    let test = {
                        let pattern = branch.regex
                                            .unshift()
                                            .expect("Invalid tree structure, please make an issue on GitHub!");

                        if pattern.weight() <= 2 {
                            quote!(#pattern =>)
                        } else {
                            let test = self.gen().pattern_to_fn(&pattern);

                            quote!(byte if #test(byte) =>)
                        }
                    };

                    let branch = match kind {
                        ForkKind::Repeat => LoopGenerator(self, PhantomData).print_branch(branch),
                        _                => self.print_branch(branch),
                    };

                    quote! { #test {
                        lex.bump();
                        #branch
                    }, }
                }).collect::<TokenStream>();

                let default = self.print_then(&mut fork.then);

                if fork.kind == ForkKind::Repeat {
                    let fallback = self.print_fallback();

                    quote!({
                        loop {
                            match lex.read() {
                                #branches
                                _ => break,
                            }

                            #fallback
                        }

                        #default
                    })
                } else {
                    quote! {
                        match lex.read() {
                            #branches
                            _ => #default,
                        }
                    }
                }
            },
        }
    }

    fn print_fallback(&mut self) -> TokenStream;
}

pub struct ExhaustiveGenerator<'a: 'b, 'b>(&'b mut Generator<'a>);
pub struct LooseGenerator<'a: 'b, 'b>(&'b mut Generator<'a>);
pub struct FallbackGenerator<'a: 'b, 'b> {
    gen: &'b mut Generator<'a>,
    boundary: Pattern,
    fallback: TokenStream,
}

pub struct LoopGenerator<'a: 'b, 'b: 'c, 'c, SubGen>(&'c mut SubGen, PhantomData<LooseGenerator<'a, 'b>>)
where
    SubGen: SubGenerator<'a> + 'c;

impl<'a, 'b> SubGenerator<'a> for ExhaustiveGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.0
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        let body = self.print_node(node);

        quote!(lex.token = #body;)
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.gen().enum_name;

        quote!(#name::#variant)
    }

    fn print_fallback(&mut self) -> TokenStream {
        quote!(return lex.token = ::logos::Logos::ERROR;)
    }
}

impl<'a, 'b> SubGenerator<'a> for LooseGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.0
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        let body = self.print_node(node);

        quote! {
            #body

            lex.token = ::logos::Logos::ERROR;
        }
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.gen().enum_name;

        quote!(return lex.token = #name::#variant)
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
        let body = self.print_node(node);
        let fallback = &self.fallback;

        quote! {
            #body

            #fallback
        }
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.gen().enum_name;
        let pattern_fn = self.gen.pattern_to_fn(&self.boundary);

        quote! {
            if !#pattern_fn(lex.read()) {
                return lex.token = #name::#variant;
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

    fn print_then(&mut self, then: &mut Option<Box<Node>>) -> TokenStream {
        if let Some(node) = then {
            self.0.print_node(&mut **node)
        } else {
            quote!(continue)
        }
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        self.0.print(node)
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        self.0.print_token(variant)
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
            Pattern::Range(from, to) => tokens.extend(quote!(#from...#to)),
            Pattern::Class(ref class) => tokens.extend(quote!(#( #class )|*)),
        }
    }
}
