use proc_macro2::TokenStream;
use quote::quote;

use crate::graph::{ByteClass, State};

use super::Generator;

impl<'a> Generator<'a> {
    /// Returns a fast loop implementation if State has an edge that points back to itself,
    /// otherwise return an empty TokenStream.
    pub fn maybe_impl_fast_loop(&mut self, state: State) -> TokenStream {
        let state_data = self.graph.get_state(state);
        let self_edge = state_data
            .normal
            .iter()
            .filter(|(_bc, next_state)| next_state == &state)
            .collect::<Vec<_>>();
        assert!(self_edge.len() <= 1, "There should only be one edge going to any given state");

        if let Some((bc, _)) = self_edge.first() {
            self.impl_fast_loop(bc)
        } else {
            TokenStream::new()
        }
    }

    /// Return a fast loop implementation for the given edge. This fast loop iterates over bytes
    /// starting at `offset` until the given edge no longer applies. Offset will now be the first
    /// offset that transitions away from the current state.
    pub fn impl_fast_loop(&mut self, self_edge: &ByteClass) -> TokenStream {
        // Note: Unlike forks, we don't ever fall back to doing normal comparisons - A LUT is always
        // generated for the loop test. Since we read multiple times, its more likely we make back
        // the time spend possibly brining the lut back into cache. I think it might be better to
        // compare if we are looking for a single byte (i.e. only one comparison operation), but
        // those are rare enough where I don't think its worth the time to optimize it.
        let (ident, loop_mask) = self.add_test_to_lut(self_edge);

        quote! {
            #[inline]
            fn loop_test(byte: u8) -> bool {
                #ident[byte as usize] & #loop_mask == 0
            }
            _fast_loop!(lex, loop_test, offset);
        }
    }
}

/// This macro is included with the generated code. It is used to manually unroll the fast_loop
/// loop.
pub fn fast_loop_macro(unroll_factor: usize) -> TokenStream {
    let index = (0..unroll_factor).collect::<Vec<_>>();

    quote! {
        macro_rules! _fast_loop {
            ($lex:ident, $test:ident, $offset:ident) => {
                // Do one bounds check for multiple bytes till EOF
                'fast_loop: {
                    while let Some(arr) = $lex.read::<&[u8; #unroll_factor]>($offset) {
                        #(if $test(arr[#index])   { $offset += #index; break 'fast_loop; })*
                        $offset += #unroll_factor;
                    }

                    while let Some(byte) = $lex.read::<u8>($offset) {
                        if $test(byte) { break 'fast_loop; }
                        $offset += 1;
                    }
                }
            };
        }
    }
}
