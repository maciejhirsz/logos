use std::collections::HashSet;
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
}

impl<'a> Generator<'a> {
    pub fn new(enum_name: &'a Ident) -> Self {
        Generator {
            enum_name,
            fns: TokenStream::new(),
            fns_check: HashSet::new(),
        }
    }

    pub fn fns(self) -> TokenStream {
        self.fns
    }

    pub fn print_tree(&mut self, strings: Vec<Token<'a, String>>, regex: Option<Token<'a, Regex>>) -> TokenStream {
        let mut strings = strings.iter();

        if let Some(item) = strings.next() {
            let mut path = ByteIter::from(item.0.as_str());
            let pattern = path.next().unwrap();

            let mut node = Node::new(pattern, &mut path, item.1);

            for item in strings {
                let mut path = ByteIter::from(item.0.as_str());
                path.next().unwrap();

                node.insert(&mut path, item.1);
            }

            if node.exhaustive() {
                ExhaustiveGenerator::print(&node, self.enum_name)
            } else {
                LooseGenerator::print(&node, self.enum_name)
            }
        } else if let Some(item) = regex {
            let handler = format!("_handle_{}", item.1).to_lowercase();
            let handler = Ident::new(&handler, Span::call_site());

            if self.fns_check.insert(item.1) {
                let mut consumers = TokenStream::new();
                let token = item.1;

                for pattern in item.0.patterns() {
                    consumers.extend(if pattern.is_repeat() {
                        quote! {
                            loop {
                                match lex.read() {
                                    #pattern => lex.bump(),
                                    _ => break,
                                }
                            }
                        }
                    } else {
                        quote! {
                            match lex.read() {
                                #pattern => lex.bump(),
                                _ => {},
                            }
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

            quote! { Some(#handler) }

        } else {
            panic!("Invalid tree!");
        }
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

pub trait GeneratorTrait {
    fn print(node: &Node, name: &Ident) -> TokenStream;

    fn print_token(name: &Ident, variant: &Ident) -> TokenStream;

    fn print_node(mut node: &Node, name: &Ident) -> TokenStream {
        let mut options = node.consequents.len();

        if options == 0 {
            return if let Some(token) = node.token {
                let token = Self::print_token(name, token);

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

                let consequent = Self::print_node(node, name);

                quote! {
                    if #test {
                        #consequent
                    }
                }
            }
            _ => {
                let branches: TokenStream = node.consequents.iter().map(|node| {
                    let pattern = &node.pattern;
                    let consequent = Self::print_node(node, name);

                    quote! { #pattern => #consequent, }
                }).collect();

                let default = match node.token {
                    Some(token) => Self::print_token(name, token),
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

pub struct LooseGenerator;
pub struct ExhaustiveGenerator;

impl GeneratorTrait for LooseGenerator {
    fn print(node: &Node, name: &Ident) -> TokenStream {
        let body = Self::print_node(node, name);

        quote! {
            Some(|lex| {
                #body

                lex.token = <#name as ::logos::Logos>::ERROR;
            })
        }
    }

    fn print_token(name: &Ident, variant: &Ident) -> TokenStream {
        quote! { return lex.token = #name::#variant }
    }
}

impl GeneratorTrait for ExhaustiveGenerator {
    fn print(node: &Node, name: &Ident) -> TokenStream {
        let body = Self::print_node(node, name);

        quote! {
            Some(|lex| {
                lex.token = #body;
            })
        }
    }

    fn print_token(name: &Ident, variant: &Ident) -> TokenStream {
        quote! { #name::#variant }
    }
}
