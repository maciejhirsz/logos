use syn::Ident;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use tree::Node;
use regex::Pattern;

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

pub trait Generator {
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

impl Generator for LooseGenerator {
    fn print(node: &Node, name: &Ident) -> TokenStream {
        let body = Self::print_node(node, name);

        quote! {
            |lex| {
                #body

                lex.token = <#name as ::logos::Logos>::ERROR;
            }
        }
    }

    fn print_token(name: &Ident, variant: &Ident) -> TokenStream {
        quote! { return lex.token = #name::#variant }
    }
}

impl Generator for ExhaustiveGenerator {
    fn print(node: &Node, name: &Ident) -> TokenStream {
        let body = Self::print_node(node, name);

        quote! {
            |lex| {
                lex.token = #body;
            }
        }
    }

    fn print_token(name: &Ident, variant: &Ident) -> TokenStream {
        quote! { #name::#variant }
    }
}

/// Tests whether the branch produces a token on all leaves without any tests.
pub fn exhaustive(node: &Node) -> bool {
    node.token.is_some() && node.consequents.iter().all(exhaustive)
}
