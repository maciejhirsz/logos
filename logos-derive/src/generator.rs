use std::collections::HashMap;
use std::rc::Rc;
use syn::Ident;
use regex::RepetitionFlag;
use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens};

use tree::{Node, Branch};
use regex::Pattern;

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
            if let Some(fallback) = tree.fallback() {
                FallbackGenerator {
                    gen: self,
                    fallback,
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

pub trait SubGenerator<'a> {
    fn gen(&mut self) -> &mut Generator<'a>;

    fn print(&mut self, node: &mut Node) -> TokenStream;

    fn print_token(&mut self, variant: &Ident) -> TokenStream;

    fn print_branch(&mut self, branch: &mut Branch) -> TokenStream {
        use self::RepetitionFlag::*;

        if branch.regex.len() == 0 {
            return self.print_node(&mut *branch.then);
        }

        match branch.regex.repeat {
            One | OneOrMore => {
                let (first, rest) = self.regex_to_test(branch.regex.consume());

                let next = self.print_branch(branch);

                quote! {
                    if #first #(&& #rest)* {
                        lex.bump();

                        #next
                    }
                }
            },
            ZeroOrMore => {
                let next = self.print_node(&mut *branch.then);
                let (first, rest) = self.regex_to_test(branch.regex.consume());

                if rest.len() == 0 {
                    quote!({
                        while #first {
                            lex.bump();
                        }

                        #next
                    })
                } else {
                    // FIXME: return with fallback here?
                    quote!({
                        let mut ok = true;

                        while #first {
                            if #(#rest)&&* {
                                lex.bump();
                            } else {
                                ok = false;

                                break;
                            }
                        }

                        if ok {
                            #next
                        }
                    })
                }
            },
            ZeroOrOne => {
                let next = self.print_node(&mut *branch.then);
                let (first, rest) = self.regex_to_test(branch.regex.consume());

                if rest.len() == 0 {
                    quote!({
                        if #first {
                            lex.bump();
                        }

                        #next
                    })
                } else {
                    // FIXME: return with fallback here?
                    quote!({
                        let mut ok = true;

                        if #first {
                            if !(#(#rest)&&*) {
                                ok = false;
                            }
                        }

                        if ok {
                            lex.bump();
                            #next
                        }
                    })
                }
            }
        }
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
            Node::Leaf(token) => self.print_token(token),
            Node::Branch(branch) => self.print_branch(branch),
            Node::Fork(fork) => {
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

                    let branch = self.print_branch(branch);

                    quote! { #test {
                        lex.bump();
                        #branch
                    }, }
                }).collect::<TokenStream>();

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

    fn print(&mut self, node: &mut Node) -> TokenStream {
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
}

impl<'a, 'b> SubGenerator<'a> for FallbackGenerator<'a, 'b> {
    fn gen(&mut self) -> &mut Generator<'a> {
        self.gen
    }

    fn print(&mut self, node: &mut Node) -> TokenStream {
        let body = self.print_node(node);
        let fallback = LooseGenerator(self.gen).print_branch(&mut self.fallback);

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
