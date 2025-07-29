use std::fmt::Write;

use fnv::FnvHashMap as Map;
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};
use syn::Ident;

use crate::graph::{Graph, State, StateType};
use crate::leaf::{Callback, InlineCallback};
use crate::util::ToIdent;

mod leaf;

pub struct Config {
    pub use_state_machine_codegen: bool,
}

pub struct Generator<'a> {
    /// Configuration for the code generation
    config: Config,
    /// Name of the type we are implementing the `Logos` trait for
    name: &'a Ident,
    /// Name of the type with any generics it might need
    this: &'a TokenStream,
    /// Reference to the graph with all of the nodes
    graph: &'a Graph,
    /// Function name identifiers
    idents: Map<State, Ident>,
    /// Callback for the default error type
    error_callback: &'a Option<Callback>,
}

impl<'a> Generator<'a> {
    pub fn new(config: Config, name: &'a Ident, this: &'a TokenStream, graph: &'a Graph, error_callback: &'a Option<Callback>) -> Self {
        let mut idents = Map::default();

        for state in graph.get_states() {
            let mut name = format!("state{}", state.dfa_id.as_usize());
            if let Some(accept) = state.context {
                write!(name, "_ctx{}", accept.0).expect("Failed to write to string");
            }
            idents.insert(state, name.to_ident());
        }

        Generator {
            config,
            name,
            this,
            graph,
            idents,
            error_callback,
        }
    }

    pub fn generate(self) -> TokenStream {
        let mut states = self.graph.get_states().collect::<Vec<_>>();
        // Sort for repeatability (not dependent on hashmap iteration order)
        states.sort_unstable();
        let states_rendered = states
            .iter()
            .map(|&state| self.generate_state(state))
            .collect::<Vec<_>>();

        let init_state = &self.idents[&self.graph.root()];
        let mut all_idents = self.idents.values().collect::<Vec<_>>();
        // Sort for repeatability (not dependent on hashmap iteration order)
        all_idents.sort_unstable();

        let error_cb = self.generate_error_cb();

        if self.config.use_state_machine_codegen {
            quote! {
                #[derive(Clone, Copy)]
                enum LogosState {
                    #(#all_idents),*
                }
                #error_cb
                let mut state = LogosState::#init_state;
                let mut offset = lex.offset();
                loop {
                    match state {
                        #(#states_rendered)*
                    }
                }
            }
        } else {
            quote! {
                #error_cb
                #(#states_rendered)*
                #init_state(lex, lex.offset())
            }
        }
    }

    fn get_ident(&self, state: &State) -> &Ident {
        self.idents.get(state).expect("Unreachable state found")
    }

    fn generate_error_cb(&self) -> TokenStream {
        let this = self.this;

        let body = match self.error_callback {
            Some(Callback::Label(label)) => quote! {
                let error = #label(lex);
                error.into()
            },
            Some(Callback::Inline(InlineCallback { arg, body, .. })) =>  quote! {
                let #arg = lex;
                let error = { #body };
                error.into()
            },
            None => quote!{
                <#this as Logos<'s>>::Error::default()
            },
        };

        quote! {
            #[inline]
            fn make_error<'s>(lex: &mut _Lexer<'s>) -> <#this as Logos<'s>>::Error {
                #body
            }
        }
    }

    fn state_transition(&self, state: &State) -> TokenStream {
        let state_ident = self.get_ident(&state);
        match self.config.use_state_machine_codegen {
            true => quote! { state = LogosState::#state_ident; },
            false => quote! { return #state_ident(lex, offset) },
        }
    }

    fn generate_state(&self, state: State) -> TokenStream {
        let this_ident = self.get_ident(&state);
        let mut setup = TokenStream::new();
        let state_data = self.graph.get_state_data(&state);

        if let StateType::Accept(_) = state_data.state_type {
            setup.append_all(quote! {
                lex.end(offset - 1);
            })
        };

        let mut inner_cases = TokenStream::new();
        for (byte_class, next_state) in &state_data.normal {
            let patterns = byte_class.ranges.iter().map(|range| {
                let start = byte_to_tokens(*range.start());
                let end = byte_to_tokens(*range.end());
                if range.len() == 1 {
                    quote! { Some(#start) }
                } else {
                    quote! { Some(#start ..= #end) }
                }
            });
            let transition = self.state_transition(next_state);
            inner_cases.append_all(quote! {
                #(#patterns)|* => {
                    offset += 1;
                    #transition
                },
            });
        }

        if state == self.graph.root() {
            // If we just started lexing and are at the end of input, return None
            inner_cases.append_all(quote! { None if lex.offset() == offset => return None, });
        }
        if let Some(eoi) = &state_data.eoi {
            let transition = self.state_transition(eoi);
            inner_cases.append_all(quote! {
                None => {
                    offset += 1;
                    #transition
                }
            });
        }

        let otherwise = if let Some(leaf_id) = state.context {
            self.generate_leaf(&self.graph.leaves()[leaf_id.0])
        } else {
            // if we reached eoi, we are already at the end of the input
            quote! {
                lex.end_to_boundary(offset + if other.is_some() { 1 } else { 0 });
                return Some(Err(make_error(lex)));
            }
        };

        let body = quote! {
            #setup
            match lex.read::<u8>(offset) {
                #inner_cases
                other => { #otherwise }
            }
        };

        if self.config.use_state_machine_codegen {
            quote! {
                LogosState::#this_ident => {
                    #body
                }
            }
        } else {
            let this = self.this;
            quote! {
                fn #this_ident<'s>(lex: &mut _Lexer<'s>, mut offset: usize)
                    -> _Option<_Result<#this, <#this as Logos<'s>>::Error>> {
                    #body
                }
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
