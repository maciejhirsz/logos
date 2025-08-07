use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};

use crate::{generator::byte_to_tokens, graph::{State, StateData}};

use super::Generator;

impl<'a> Generator<'a> {

    pub fn impl_fork(&mut self, state: State, state_data: &StateData) -> TokenStream {
        self.impl_fork_match(state, state_data)
    }


    fn impl_fork_match(&mut self, state: State, state_data: &StateData) -> TokenStream {
        // Generate a match arm for each byte class, with each body being a state transition
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

        // Add special handling for end of input, both within a token and between them
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

        // If nothing else applies, return the current token (active context)
        // or an error (no active context).
        let otherwise = if let Some(leaf_id) = state.context {
            self.generate_leaf(&self.graph.leaves()[leaf_id.0])
        } else {
            // if we reached eoi, we are already at the end of the input
            // so don't add 1 to offset.
            quote! {
                lex.end_to_boundary(offset + if other.is_some() { 1 } else { 0 });
                return Some(Err(_make_error(lex)));
            }
        };

        quote! {
            match lex.read::<u8>(offset) {
                #inner_cases
                other => { #otherwise }
            }
        }
    }

}
