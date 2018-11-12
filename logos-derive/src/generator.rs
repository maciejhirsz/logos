use std::collections::{HashSet, HashMap};
use syn::Ident;
use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens};

use tree::Node;
use regex::{Regex, Pattern, ByteIter};
use handlers::{Branch, Tree};

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

    pub fn print_tree(&mut self, tree: Tree<'a>) -> TokenStream {
        let mut strings = tree.strings.iter();

        if let Some(branch) = strings.next() {
            let mut path = ByteIter::from(branch.0.as_str());
            let pattern = path.next().unwrap();

            let mut node = Node::new(pattern, &mut path, branch.1);

            for branch in strings {
                let mut path = ByteIter::from(branch.0.as_str());
                path.next().unwrap();

                node.insert(&mut path, branch.1);
            }

            if let Some(fallback) = tree.regex {
                FallbackGenerator {
                    gen: self,
                    fallback,
                }.print(&node)
            } else if node.exhaustive() {
                ExhaustiveGenerator(self.enum_name).print(&node)
            } else {
                LooseGenerator(self.enum_name).print(&node)
            }
        } else if let Some(regex) = tree.regex {
            let handler = self.regex_to_fn(regex);
            quote! {
                Some(#handler)
            }
        } else {
            panic!("Invalid tree!");
        }
    }

    fn regex_to_fn(&mut self, branch: Branch<'a, Regex>) -> Ident {
        let handler = format!("_handle_{}", branch.1).to_lowercase();
        let handler = Ident::new(&handler, Span::call_site());

        if self.fns_check.insert(branch.1) {
            let mut consumers = TokenStream::new();
            let token = branch.1;

            for pattern in branch.0.patterns() {
                let pattern_fn = self.pattern_to_fn(pattern.clone());

                let tokens = if pattern.is_repeat() {
                    quote! {
                        while #pattern_fn(lex.read()) {
                            lex.bump();
                        }
                    }
                } else {
                    quote! {
                        if !#pattern_fn(lex.read()) {
                            return lex.token = ::logos::Logos::ERROR;
                        }

                        lex.bump();
                    }
                };

                consumers.extend(tokens);
            }

            let name = self.enum_name;

            self.fns.extend(quote! {
                fn #handler<S: ::logos::Source>(lex: &mut Lexer<S>) {
                    lex.bump();

                    #consumers

                    lex.token = #name::#token;
                }
            });
        }

        handler
    }

    fn pattern_to_fn(&mut self, pattern: Pattern) -> &Ident {
        let idx = self.patterns.len();

        let patterns = &mut self.patterns;
        let fns = &mut self.fns;

        patterns.entry(pattern.clone()).or_insert_with(|| {
            let bytes: Vec<u8> = pattern.collect();
            let chars = bytes.iter()
                             .filter(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                             .map(|b| *b as char)
                             .collect::<String>();
            let pattern = Ident::new(&format!("_pattern_{}_{}", chars, idx), Span::call_site());

            let tokens = if bytes.len() == 1 {
                let byte = bytes[0];

                quote! {
                    #[inline]
                    fn #pattern(byte: u8) -> bool {
                        byte == #byte
                    }
                }
            } else {
                let mut table = [false; 256];

                for byte in bytes {
                    table[byte as usize] = true;
                }

                let ltrue = quote!(TT);
                let lfalse = quote!(__);

                let table = table.iter().map(|x| if *x { &ltrue } else { &lfalse });

                quote! {
                    #[inline]
                    fn #pattern(byte: u8) -> bool {
                        const #ltrue: bool = true;
                        const #lfalse: bool = false;

                        static LUT: [bool; 256] = [#( #table ),*];

                        LUT[byte as usize]
                    }
                }
            };

            fns.extend(tokens);

            pattern
        })
    }

    pub fn fns(self) -> TokenStream {
        self.fns
    }
}

pub trait GeneratorTrait<'a> {
    fn enum_name(&self) -> &'a Ident;

    fn print(&mut self, node: &Node) -> TokenStream;

    fn print_token(&mut self, variant: &Ident) -> TokenStream;

    fn print_node(&mut self, mut node: &Node) -> TokenStream {
        let mut options = node.consequents.len();

        if options == 0 {
            return if let Some(token) = node.token {
                let token = self.print_token(token);

                quote! { {
                    lex.bump();

                    #token
                } }
            } else {
                TokenStream::new()
            };
        }

        if node.token.is_some() || node.consequents.first().map(|node| !node.pattern.is_byte()).unwrap_or(false)  {
            options += 1;
        }

        match options {
            1 => {
                node = node.consequents.first().unwrap();
                let mut test = vec![&node.pattern];

                while node.consequents.len() == 1 && node.token.is_none() && node.consequents.first().unwrap().pattern.is_byte() {
                    node = node.consequents.iter().next().unwrap();

                    test.push(&node.pattern);
                }

                let consequent = self.print_node(node);

                quote! {
                    if #( lex.next() == #test )&&* {
                        #consequent
                    }
                }
            }
            _ => {
                let branches: TokenStream = node.consequents.iter().map(|node| {
                    let pattern = &node.pattern;
                    let consequent = self.print_node(node);

                    quote! { #pattern => #consequent, }
                }).collect();

                let default = match node.token {
                    Some(token) => self.print_token(token),
                    None        => quote! { {} },
                };

                quote! {
                    match lex.next() {
                        #branches
                        _ => #default,
                    }
                }
            }
        }
    }
}

pub struct ExhaustiveGenerator<'a>(&'a Ident);
pub struct LooseGenerator<'a>(&'a Ident);
pub struct FallbackGenerator<'a: 'b, 'b> {
    gen: &'b mut Generator<'a>,
    fallback: Branch<'a, Regex>,
}

impl<'a> GeneratorTrait<'a> for ExhaustiveGenerator<'a> {
    fn enum_name(&self) -> &'a Ident {
        self.0
    }

    fn print(&mut self, node: &Node) -> TokenStream {
        let body = self.print_node(node);

        quote! {
            Some(|lex| {
                lex.token = #body;
            })
        }
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.enum_name();

        quote! { #name::#variant }
    }
}

impl<'a> GeneratorTrait<'a> for LooseGenerator<'a> {
    fn enum_name(&self) -> &'a Ident {
        self.0
    }

    fn print(&mut self, node: &Node) -> TokenStream {
        let body = self.print_node(node);

        quote! {
            Some(|lex| {
                #body

                lex.token = ::logos::Logos::ERROR;
            })
        }
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.enum_name();

        quote! { return lex.token = #name::#variant }
    }
}

impl<'a, 'b> GeneratorTrait<'a> for FallbackGenerator<'a, 'b> {
    fn enum_name(&self) -> &'a Ident {
        self.gen.enum_name
    }

    fn print(&mut self, node: &Node) -> TokenStream {
        let body = self.print_node(node);
        let pattern = &self.fallback.0.patterns()[0];
        let pattern_fn = self.gen.pattern_to_fn(pattern.clone());

        if !pattern.is_repeat() {
            panic!("Sorry, you are trying to do something that's not implemented yet!");
        };

        let name = self.gen.enum_name.clone();
        let fallback = &self.fallback.1;

        quote! {
            Some(|lex| {
                #body

                while #pattern_fn(lex.read()) {
                    lex.bump();
                }

                lex.token = #name::#fallback;
            })
        }
    }

    fn print_token(&mut self, variant: &Ident) -> TokenStream {
        let name = self.enum_name();
        let pattern = &self.fallback.0.patterns()[0];
        let pattern_fn = self.gen.pattern_to_fn(pattern.clone());

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
            Pattern::Repeat(ref pat) => pat.to_tokens(tokens),
            Pattern::Alternative(ref pat) => tokens.extend(quote!(#( #pat )|*)),
        }
    }
}
