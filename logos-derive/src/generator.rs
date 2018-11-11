use std::collections::{HashSet, HashMap};
use syn::Ident;
use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens};

use tree::Node;
use regex::{Regex, Pattern, ByteIter};
use handlers::Token;

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

    pub fn print_tree(&mut self, strings: Vec<Token<'a, String>>, regex: Option<Token<'a, Regex>>) -> TokenStream {
        let mut strings = strings.iter();

        let regex = regex.map(|regex| self.regex_to_fn(regex));

        if let Some(item) = strings.next() {
            let mut path = ByteIter::from(item.0.as_str());
            let pattern = path.next().unwrap();

            let mut node = Node::new(pattern, &mut path, item.1);

            for item in strings {
                let mut path = ByteIter::from(item.0.as_str());
                path.next().unwrap();

                node.insert(&mut path, item.1);
            }

            if let Some(fallback) = regex {
                FallbackGenerator {
                    enum_name: self.enum_name,
                    fallback,
                }.print(&node)
            } else if node.exhaustive() {
                ExhaustiveGenerator(self.enum_name).print(&node)
            } else {
                LooseGenerator(self.enum_name).print(&node)
            }
        } else if let Some(handler) = regex {
            quote! {
                Some(#handler)
            }
        } else {
            panic!("Invalid tree!");
        }
    }

    fn regex_to_fn(&mut self, item: Token<'a, Regex>) -> Ident {
        let handler = format!("_handle_{}", item.1).to_lowercase();
        let handler = Ident::new(&handler, Span::call_site());

        if self.fns_check.insert(item.1) {
            let mut consumers = TokenStream::new();
            let token = item.1;

            for pattern in item.0.patterns() {
                let pattern_fn = self.pattern_to_fn(pattern.clone());
                let if_or_while = if pattern.is_repeat() { quote!(while) } else { quote!(if) };

                consumers.extend(quote! {
                    #if_or_while #pattern_fn(lex.read()) {
                        lex.bump();
                    }
                });
            }

            let name = self.enum_name;

            self.fns.extend(quote! {
                fn #handler<S: ::logos::Source>(lex: &mut ::logos::Lexer<#name, S>) {
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

    fn print(&self, node: &Node) -> TokenStream;

    fn print_token(&self, variant: &Ident) -> TokenStream;

    fn print_node(&self, mut node: &Node) -> TokenStream {
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
                let byte = &node.pattern;

                let mut test = quote! { lex.next() == #byte };

                while node.consequents.len() == 1 && node.token.is_none() && node.consequents.first().unwrap().pattern.is_byte() {
                    node = node.consequents.iter().next().unwrap();
                    let byte = &node.pattern;

                    test.extend(quote! { && lex.next() == #byte });
                }

                let consequent = self.print_node(node);

                quote! {
                    if #test {
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
pub struct FallbackGenerator<'a> {
    enum_name: &'a Ident,
    fallback: Ident,
}

impl<'a> GeneratorTrait<'a> for ExhaustiveGenerator<'a> {
    fn enum_name(&self) -> &'a Ident {
        self.0
    }

    fn print(&self, node: &Node) -> TokenStream {
        let body = self.print_node(node);

        quote! {
            Some(|lex| {
                lex.token = #body;
            })
        }
    }

    fn print_token(&self, variant: &Ident) -> TokenStream {
        let name = self.enum_name();

        quote! { #name::#variant }
    }
}

impl<'a> GeneratorTrait<'a> for LooseGenerator<'a> {
    fn enum_name(&self) -> &'a Ident {
        self.0
    }

    fn print(&self, node: &Node) -> TokenStream {
        let body = self.print_node(node);

        quote! {
            Some(|lex| {
                #body

                lex.token = ::logos::Logos::ERROR;
            })
        }
    }

    fn print_token(&self, variant: &Ident) -> TokenStream {
        let name = self.enum_name();

        quote! { return lex.token = #name::#variant }
    }
}

impl<'a> GeneratorTrait<'a> for FallbackGenerator<'a> {
    fn enum_name(&self) -> &'a Ident {
        self.enum_name
    }

    fn print(&self, node: &Node) -> TokenStream {
        let body = self.print_node(node);
        let fallback = &self.fallback;

        quote! {
            Some(|lex| {
                #body

                #fallback(lex);
            })
        }
    }

    fn print_token(&self, variant: &Ident) -> TokenStream {
        let name = self.enum_name();

        quote! { return lex.token = #name::#variant }
    }
}

impl ToTokens for Pattern {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Pattern::Byte(byte) => tokens.extend(quote! { #byte }),
            Pattern::Range(from, to) => tokens.extend(quote! { #from...#to }),
            Pattern::Repeat(ref pat) => pat.to_tokens(tokens),
            Pattern::Alternative(ref pat) => tokens.extend(quote! { #( #pat )|* }),
        }
    }
}
