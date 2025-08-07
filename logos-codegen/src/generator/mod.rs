use fnv::FnvHashMap as Map;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::graph::{Graph, State, StateType};
use crate::leaf::{Callback, InlineCallback};
use crate::util::ToIdent;

mod fork;
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
    /// Mapping of states to their identifiers.
    /// Identifiers are enum variants in the state machine codegen
    /// and function names in the tailcall codegen.
    idents: Map<State, Ident>,
    /// Callback for the default error type
    error_callback: &'a Option<Callback>,
}

impl<'a> Generator<'a> {
    pub fn new(
        config: Config,
        name: &'a Ident,
        this: &'a TokenStream,
        graph: &'a Graph,
        error_callback: &'a Option<Callback>,
    ) -> Self {
        let idents = graph
            .get_states()
            .map(|state| (state, state.to_string().to_ident()))
            .collect();

        Generator {
            config,
            name,
            this,
            graph,
            idents,
            error_callback,
        }
    }

    /// Generates the implementation (body) of the [Logos::lex] function
    pub fn generate(&mut self) -> TokenStream {
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

    // Generates the definition for the `_make_error` function. This can be
    // specified using the `callback` argument of the `error` attribute.
    // Otherwise, it defaults to the `Default::default()`value.
    fn generate_error_cb(&self) -> TokenStream {
        let this = self.this;

        let body = match self.error_callback {
            Some(Callback::Label(label)) => quote! {
                let error = #label(lex);
                error.into()
            },
            Some(Callback::Inline(InlineCallback { arg, body, .. })) => quote! {
                let #arg = lex;
                let error = { #body };
                error.into()
            },
            None => quote! {
                <#this as Logos<'s>>::Error::default()
            },
        };

        quote! {
            #[inline]
            fn _make_error<'s>(lex: &mut _Lexer<'s>) -> <#this as Logos<'s>>::Error {
                #body
            }
        }
    }

    /// Generates the code to transition to a state.
    fn state_transition(&self, state: &State) -> TokenStream {
        return self.state_action(self.state_value(state));
    }

    /// Generates the code to transition to a state stored in an identifier
    fn state_action(&self, state_ident: TokenStream) -> TokenStream {
        match self.config.use_state_machine_codegen {
            true => quote! { state = #state_ident; continue; },
            false => quote! { return #state_ident(lex, offset); },
        }
    }

    /// Generates the code to quote a state's representation
    fn state_value(&self, state: &State) -> TokenStream {
        let state_ident = self.get_ident(&state);
        match self.config.use_state_machine_codegen {
            true => quote!(LogosState::#state_ident),
            false => quote!(#state_ident),
        }
    }

    /// Generates the body of a state. This is a match statement over
    /// the next byte, which determines the next state.
    ///
    /// It also instantiates the relevant leaf, if `state` has a context.
    ///
    /// In state machine codegen, the body is wrapped in a match arm for the
    /// `state`'s variant. In tailcall codegen, the body is inside of
    /// `state`'s function.
    fn generate_state(&mut self, state: State) -> TokenStream {
        let state_data = self.graph.get_state_data(&state);

        // If we are in a match state, update the current token to
        // end at the current offset - 1.
        // The 1 comes from the 1 byte delayed match behavior
        // of the regex-automata crate.
        let setup = if let StateType::Accept(_) = state_data.state_type {
            Some(quote!(lex.end(offset - 1);))
        } else {
            None
        };

        let body = self.impl_fork(state, state_data);

        // Wrap body in a match arm or function depending on the current codegen
        let this_ident = self.get_ident(&state);
        if self.config.use_state_machine_codegen {
            quote! {
                LogosState::#this_ident => {
                    #setup
                    #body
                }
            }
        } else {
            let this = self.this;
            quote! {
                fn #this_ident<'s>(lex: &mut _Lexer<'s>, mut offset: usize)
                    -> _Option<_Result<#this, <#this as Logos<'s>>::Error>> {
                    #setup
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

/// Converts a byte to a byte literal that can be used to match it
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
