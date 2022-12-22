use proc_macro2::{Literal, TokenStream};
use quote::{quote, ToTokens as _};

use crate::generator::{Context, Generator};
use crate::graph::Rope;
use crate::util::is_ascii;

impl<'a> Generator<'a> {
    pub fn generate_rope(&mut self, rope: &Rope, mut ctx: Context) -> TokenStream {
        let miss = ctx.miss(rope.miss.first(), self);
        let read = ctx.read(rope.pattern.len());
        let then = self.goto(rope.then, ctx.advance(rope.pattern.len()));

        let pat = match rope.pattern.to_bytes() {
            Some(bytes) => byte_slice_literal(&bytes),
            None => {
                let ranges = rope.pattern.iter();

                quote!([#(#ranges),*])
            }
        };

        quote! {
            match #read {
                Some(#pat) => #then,
                _ => #miss,
            }
        }
    }
}

fn byte_slice_literal(bytes: &[u8]) -> TokenStream {
    if bytes.iter().copied().all(is_ascii) {
        Literal::byte_string(bytes).into_token_stream()
    } else {
        quote!(&[#(#bytes),*])
    }
}
