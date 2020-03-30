use proc_macro2::TokenStream;
use quote::quote;
use fnv::FnvHashMap as Map;

use crate::graph::{NodeId, Fork, Range};
use crate::generator::{Generator, Context};

impl<'a> Generator<'a> {
    pub fn generate_fork(&mut self, this: NodeId, fork: &Fork, ctx: Context) -> TokenStream {
        let mut targets: Map<NodeId, Vec<Range>> = Map::default();

        for (range, then) in fork.branches() {
            targets.entry(then).or_default().push(range);
        }

        if self.meta[&this].loop_entry_from.contains(&this) && targets.len() == 1 {
            return self.generate_fast_loop(fork, ctx);
        }
        let miss = ctx.miss(fork.miss, self);
        let end = if this == self.root {
            quote!(_end(lex))
        } else {
            miss.clone()
        };
        let read = match ctx.at {
            0 => quote!(lex.read()),
            n => quote!(lex.read_at(#n)),
        };
        let branches = targets.into_iter().map(|(id, ranges)| {
            match *ranges {
                [range] => {
                    let next = self.goto(id, ctx.push(1));
                    quote!(#range => #next,)
                },
                _ => {
                    let test = self.generate_test(ranges).clone();
                    let next = self.goto(id, ctx.push(1));

                    quote!(byte if #test(byte) => #next,)
                },
            }
        });

        quote! {
            let byte = match #read {
                Some(byte) => byte,
                None => return #end,
            };

            match byte {
                #(#branches)*
                _ => #miss,
            }
        }
    }

    fn generate_fast_loop(&mut self, fork: &Fork, ctx: Context) -> TokenStream {
        let miss = ctx.miss(fork.miss, self);
        let ranges = fork.branches().map(|(range, _)| range).collect::<Vec<_>>();
        let test = self.generate_test(ranges);

        quote! {
            _fast_loop!(lex, #test, #miss);
        }
    }

    pub fn fast_loop_macro() -> TokenStream {
        quote! {
            macro_rules! _fast_loop {
                ($lex:ident, $test:ident, $miss:expr) => {
                    // Do one bounds check for multiple bytes till EOF
                    while let Some(arr) = $lex.read::<&[u8; 8]>() {
                        if $test(arr[0]) { if $test(arr[1]) { if $test(arr[2]) { if $test(arr[3]) {
                        if $test(arr[4]) { if $test(arr[5]) { if $test(arr[6]) { if $test(arr[7]) {

                        $lex.bump(8); continue;     } $lex.bump(7); return $miss; }
                        $lex.bump(6); return $miss; } $lex.bump(5); return $miss; }
                        $lex.bump(4); return $miss; } $lex.bump(3); return $miss; }
                        $lex.bump(2); return $miss; } $lex.bump(1); return $miss; }

                        return $miss;
                    }

                    while $lex.test($test) {
                        $lex.bump(1);
                    }

                    $miss
                };
            }
        }
    }
}
