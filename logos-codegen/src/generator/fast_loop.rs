use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{graph::{ByteClass, State}, util::ToIdent};

use super::Generator;

fn loop_table_ident(index: usize) -> Ident {
    format!("_LOOP_TABLE_{index}").to_ident()
}

impl<'a> Generator<'a> {
    pub fn maybe_impl_fast_loop(&mut self, state: State) -> TokenStream {
        let state_data = self.graph.get_state(state);
        let self_edge = state_data
            .normal
            .iter()
            .filter(|(_bc, next_state)| next_state == &state)
            .collect::<Vec<_>>();
        assert!(self_edge.len() <= 1);

        if let Some((bc, _)) = self_edge.first() {
            self.impl_fast_loop(bc)
        } else {
            TokenStream::new()
        }
    }

    /// TODO
    pub fn impl_fast_loop(&mut self, self_edge: &ByteClass) -> TokenStream {
        // TODO: generate loop test as a comparison if it is simple
        let loop_id = self.loop_masks.len();
        let loop_table = loop_id / 8;
        let ident = loop_table_ident(loop_table);
        let loop_mask = 1u8 << (loop_id % 8);

        let mut table_bits = [false; 256];
        for range in self_edge.ranges.iter() {
            for byte in range.clone() {
                table_bits[byte as usize] = true;
            }
        }

        self.loop_masks.push(table_bits);

        quote! {
            #[inline]
            fn loop_test(byte: u8) -> bool {
                #ident[byte as usize] & #loop_mask == 0
            }
            _fast_loop!(lex, loop_test, offset);
        }
    }

    pub fn render_loop_luts(&self) -> TokenStream {
        TokenStream::from_iter(
            self.loop_masks
                .chunks(8)
                .enumerate()
                .map(|(lut_idx, bit_arrs)| {
                    let mut byte_arr = [0u8; 256];
                    for (bit_index, bits) in bit_arrs.iter().enumerate() {
                        for (arr_idx, &bit) in bits.iter().enumerate() {
                            if bit {
                                byte_arr[arr_idx] |= 1 << bit_index;
                            }
                        }
                    }

                    let ident = loop_table_ident(lut_idx);
                    quote! { const #ident: [u8; 256] = [#(#byte_arr),*]; }
                }),
        )
    }
}

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
