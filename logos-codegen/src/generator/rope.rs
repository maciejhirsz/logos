use proc_macro2::TokenStream;
use quote::quote;

use crate::generator::{Context, Generator};
use crate::graph::Rope;

impl Generator<'_> {
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
    if bytes.iter().any(|&b| !(0x20..0x7F).contains(&b)) {
        return quote!(&[#(#bytes),*]);
    }

    let slice = std::str::from_utf8(bytes).unwrap();

    syn::parse_str(&format!("b{:?}", slice)).unwrap()
}
