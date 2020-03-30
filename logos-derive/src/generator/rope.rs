use proc_macro2::TokenStream;
use quote::quote;

use crate::graph::Rope;
use crate::generator::{Generator, Context};

impl<'a> Generator<'a> {
    pub fn generate_rope(&mut self, rope: &Rope, ctx: Context) -> TokenStream {
        let miss = ctx.miss(rope.miss.first(), self);
        let len = rope.pattern.len();
        let then = self.goto(rope.then, ctx.push(rope.pattern.len()));
        let read = match ctx.at {
            0 => quote!(lex.read::<&[u8; #len]>()),
            n => quote!(lex.read_at::<&[u8; #len]>(#n)),
        };

        if let Some(bytes) = rope.pattern.to_bytes() {
            let pat = byte_slice_literal(&bytes);

            return quote! {
                match #read {
                    Some(#pat) => #then,
                    _ => #miss,
                }
            };
        }

        let matches = rope.pattern.iter().enumerate().map(|(idx, range)| {
            quote! {
                match bytes[#idx] {
                    #range => (),
                    _ => return #miss,
                }
            }
        });

        quote! {
            match #read {
                Some(bytes) => {
                    #(#matches)*

                    #then
                },
                None => #miss,
            }
        }
    }
}

fn byte_slice_literal(bytes: &[u8]) -> TokenStream {
    if bytes.iter().any(|&b| b < 0x20 || b >= 0x7F) {
        return quote!(&[#(#bytes),*]);
    }

    let slice = std::str::from_utf8(bytes).unwrap();

    syn::parse_str(&format!("b{:?}", slice)).unwrap()
}