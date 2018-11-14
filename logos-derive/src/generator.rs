use std::collections::{HashSet, HashMap};
use syn::Ident;
use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens};

use tree::{Node, Branch};
use regex::Pattern;

pub struct Generator<'a> {
    enum_name: &'a Ident,
    fns: TokenStream,
    fns_check: HashSet<&'a Ident>,
    patterns: HashMap<Pattern, Ident>,
}

impl<'a> Generator<'a> {
    pub fn new(enum_name: &'a Ident) -> Self {
        Generator {
            enum_name,
            fns: TokenStream::new(),
            fns_check: HashSet::new(),
            patterns: HashMap::new(),
        }
    }

    pub fn print_tree(&mut self, tree: Node<'a>) -> TokenStream {
        match tree.only_leaf() {
            Some(variant) => {
                let handler = format!("_handle_{}", variant).to_lowercase();
                let handler = Ident::new(&handler, Span::call_site());

                if self.fns_check.insert(variant) {
                    let body = self.tree_to_fn_body(tree);

                    self.fns.extend(quote! {
                        fn #handler<S: ::logos::Source>(lex: &mut Lexer<S>) {
                            lex.bump();
                            #body
                        }
                    });
                }

                quote!(Some(#handler))
            },
            None => {
                let body = self.tree_to_fn_body(tree);

                quote!(Some({fn handler<S: ::logos::Source>(lex: &mut Lexer<S>) {
                    lex.bump();
                    #body
                } handler}))
            }
        }
    }

    fn tree_to_fn_body(&mut self, mut tree: Node<'a>) -> TokenStream {
        if tree.exhaustive() {
            ExhaustiveGenerator(self).print(&tree)
        } else {
            if let Some(fallback) = tree.fallback() {
                FallbackGenerator {
                    gen: self,
                    fallback,
                }.print(&tree)
            } else {
                LooseGenerator(self).print(&tree)
            }
        }
    }

    fn regex_to_consumers(&mut self, mut regex: &[Pattern], then: TokenStream) -> TokenStream {
        let mut tokens = TokenStream::new();

        while regex.len() != 0 {
            let bytes = regex.iter().take_while(|pat| pat.is_byte()).count();

            if bytes != 0 {
                let first = &regex[0];
                let rest = &regex[1..bytes];

                regex = &regex[bytes..];

                if regex.len() == 0 {
                    tokens.extend(quote! {
                        if lex.read() == #first #(&& lex.next() == #rest )* {
                            lex.bump();
                            #then
                        }
                    });

                    return quote!({ #tokens });
                } else {
                    tokens.extend(quote! {
                        if lex.read() != #first #(|| lex.next() != #rest )* {
                            // FIXME: Need to handle fallback in FallbackGenerator here
                            return lex.token = ::logos::Logos::ERROR;
                        }
                        lex.bump();
                    });
                }
            }

            if regex[0].is_repeat() {
                let test = self.pattern_to_fn(&regex[0]);

                if regex[0].is_repeat_plus() {
                    tokens.extend(quote! {
                        if #test(lex.read()) {
                            lex.bump();
                        } else {
                            // FIXME: Need to handle fallback in FallbackGenerator here
                            return lex.token = ::logos::Logos::ERROR;
                        }
                    });
                }

                tokens.extend(quote! {
                    while #test(lex.read()) {
                        lex.bump();
                    }
                });

                regex = &regex[1..];

                continue;
            }

            break;
        }

        quote!(
            #tokens

            #then
        )
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
                    let bytes: Vec<u8> = pattern.clone().collect();

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

pub trait SubGenerator<'a> {
    fn gen(&mut self) -> &mut Generator<'a>;

    fn print(&mut self, node: &Node) -> TokenStream;

    fn print_token(&mut self, variant: &Ident) -> TokenStream;

    fn print_branch(&mut self, branch: &Branch) -> TokenStream {
        let consequent = self.print_node(&*branch.then);

        self.gen().regex_to_consumers(branch.regex.patterns(), consequent)
    }

    fn print_node(&mut self, node: &Node) -> TokenStream {
        match node {
            Node::Leaf(token) => self.print_token(token),
            Node::Branch(branch) => {
                let branch = self.print_branch(branch);

                quote!({
                    #branch
                })
            },
            Node::Fork(fork) => {
                let branches: TokenStream = fork.arms.iter().map(|branch| {
                    let pattern = &branch.regex.first();

                    let test = if pattern.is_byte() {
                        quote!(#pattern =>)
                    } else {
                        let test = self.gen().pattern_to_fn(pattern);

                        quote!(byte if #test(byte) =>)
                    };
                    let regex = if pattern.is_repeat() {
                        branch.regex.patterns()
                    } else {
                        &branch.regex.patterns()[1..]
                    };

                    let consequent = self.print_node(&*branch.then);
                    let consequent = self.gen().regex_to_consumers(regex, consequent);

                    quote! { #test {
                        lex.bump();
                        #consequent
                    }, }
                }).collect();

                let default = match fork.default {
                    Some(token) => self.print_token(token),
                    None        => quote! { {} },
                };

                quote! {
                    match lex.read() {
                        #branches
                        _ => #default,
                    }
                }
            },
        }
    }
}

pub struct ExhaustiveGenerator<'a: 'b, 'b>(&'b mut Generator<'a>);
pub struct LooseGenerator<'a: 'b, 'b>(&'b mut Generator<'a>);
pub struct FallbackGenerator<'a: 'b, 'b> {
    gen: &'b mut Generator<'a>,
    fallback: Branch<'a>,
}

impl<'a, 'b> SubGenerator<'a> for ExhaustiveGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.0
    }

    fn print(&mut self, node: &Node) -> TokenStream {
        let body = self.print_node(node);

        quote!(lex.token = #body;)
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.gen().enum_name;

        quote!(#name::#variant)
    }
}

impl<'a, 'b> SubGenerator<'a> for LooseGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.0
    }

    fn print(&mut self, node: &Node) -> TokenStream {
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
}

impl<'a, 'b> SubGenerator<'a> for FallbackGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.gen
    }

    fn print(&mut self, node: &Node) -> TokenStream {
        let body = self.print_node(node);
        let fallback = LooseGenerator(self.gen).print_branch(&self.fallback);

        quote! {
            #body

            #fallback
        }
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.gen().enum_name;
        let pattern = self.fallback.regex.first();
        let pattern_fn = self.gen.pattern_to_fn(pattern);

        quote! {
            if !#pattern_fn(lex.read()) {
                return lex.token = #name::#variant;
            }
        }
    }
}

impl ToTokens for Pattern {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            // This is annoying, but it seems really hard to make quote!
            // print byte chars instead of integers otherwise
            Pattern::Byte(byte) => tokens.extend(match *byte {
                b'0' => quote!(b'0'),
                b'1' => quote!(b'1'),
                b'2' => quote!(b'2'),
                b'3' => quote!(b'3'),
                b'4' => quote!(b'4'),
                b'5' => quote!(b'5'),
                b'6' => quote!(b'6'),
                b'7' => quote!(b'7'),
                b'8' => quote!(b'8'),
                b'9' => quote!(b'9'),
                b'a' => quote!(b'a'),
                b'b' => quote!(b'b'),
                b'c' => quote!(b'c'),
                b'd' => quote!(b'd'),
                b'e' => quote!(b'e'),
                b'f' => quote!(b'f'),
                b'g' => quote!(b'g'),
                b'h' => quote!(b'h'),
                b'i' => quote!(b'i'),
                b'j' => quote!(b'j'),
                b'k' => quote!(b'k'),
                b'l' => quote!(b'l'),
                b'm' => quote!(b'm'),
                b'n' => quote!(b'n'),
                b'o' => quote!(b'o'),
                b'p' => quote!(b'p'),
                b'q' => quote!(b'q'),
                b'r' => quote!(b'r'),
                b's' => quote!(b's'),
                b't' => quote!(b't'),
                b'u' => quote!(b'u'),
                b'v' => quote!(b'v'),
                b'w' => quote!(b'w'),
                b'x' => quote!(b'x'),
                b'y' => quote!(b'y'),
                b'z' => quote!(b'z'),
                _    => quote!(#byte),
            }),
            Pattern::Range(from, to) => tokens.extend(quote!(#from...#to)),
            Pattern::Flagged(ref pat, _) => pat.to_tokens(tokens),
            Pattern::Alternative(ref pat) => tokens.extend(quote!(#( #pat )|*)),
        }
    }
}
