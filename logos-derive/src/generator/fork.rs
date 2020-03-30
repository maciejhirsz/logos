use proc_macro2::{TokenStream, Literal};
use quote::quote;
use fnv::FnvHashMap as Map;

use crate::graph::{NodeId, Fork, Range};
use crate::generator::{Generator, Context};

type Targets = Map<NodeId, Vec<Range>>;

impl<'a> Generator<'a> {
    pub fn generate_fork(&mut self, this: NodeId, fork: &Fork, mut ctx: Context) -> TokenStream {
        let mut targets: Targets = Map::default();

        for (range, then) in fork.branches() {
            targets.entry(then).or_default().push(range);
        }
        let loops_to_self = self.meta[&this].loop_entry_from.contains(&this);

        match targets.len() {
            1 if loops_to_self => return self.generate_fast_loop(fork, ctx),
            0..=2 => (),
            _ => return self.generate_fork_jump_table(this, fork, targets, ctx),
        }
        let miss = ctx.miss(fork.miss, self);
        let end = self.fork_end(this, &miss);
        let (byte, read) = self.fork_read(this, end, &mut ctx);
        let branches = targets.into_iter().map(|(id, ranges)| {
            match *ranges {
                [range] => {
                    let next = self.goto(id, ctx.advance(1));
                    quote!(#range => #next,)
                },
                _ => {
                    let test = self.generate_test(ranges).clone();
                    let next = self.goto(id, ctx.advance(1));

                    quote!(byte if #test(byte) => #next,)
                },
            }
        });

        quote! {
            #read

            match #byte {
                #(#branches)*
                _ => #miss,
            }
        }
    }

    fn generate_fork_jump_table(&mut self, this: NodeId, fork: &Fork, targets: Targets, mut ctx: Context) -> TokenStream {
        let miss = ctx.miss(fork.miss, self);
        let end = self.fork_end(this, &miss);
        let (byte, read) = self.fork_read(this, end, &mut ctx);

        let mut table: [u8; 256] = [0; 256];

        let branches = targets.into_iter().enumerate().map(|(idx, (id, ranges))| {
            let idx = (idx as u8) + 1;
            let next = self.goto(id, ctx.advance(1));

            for byte in ranges.into_iter().flatten() {
                table[byte as usize] = idx;
            }
            let idx = Literal::u8_unsuffixed(idx);

            quote!(#idx => #next,)
        }).collect::<TokenStream>();

        let table = table.iter().copied().map(Literal::u8_unsuffixed);

        quote! {
            const LUT: [u8; 256] = [#(#table),*];

            #read

            match LUT[#byte as usize] {
                #branches
                _ => #miss,
            }
        }
    }

    fn fork_end(&self, this: NodeId, miss: &TokenStream) -> TokenStream {
        if this == self.root {
            quote!(_end(lex))
        } else {
            miss.clone()
        }
    }

    fn fork_read(&self, this: NodeId, end: TokenStream, ctx: &mut Context) -> (TokenStream, TokenStream) {
        let min_read = self.meta[&this].min_read;

        if ctx.remainder() >= min_read {
            let at = ctx.at();

            return (
                quote!(arr[#at]),
                quote!(),
            );
        }

        match self.meta[&this].min_read {
            0 | 1 => {
                let read = ctx.read(0);

                (
                    quote!(byte),
                    quote! {
                        let byte = match #read {
                            Some(byte) => byte,
                            None => return #end,
                        };
                    },
                )
            },
            len => {
                let read = ctx.read(len);

                (
                    quote!(arr[0]),
                    quote! {
                        let arr = match #read {
                            Some(arr) => arr,
                            None => return #end,
                        };
                    },
                )
            },
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
                    while let Some(arr) = $lex.read::<&[u8; 16]>() {
                        if $test(arr[0])  { if $test(arr[1])  { if $test(arr[2])  { if $test(arr[3]) {
                        if $test(arr[4])  { if $test(arr[5])  { if $test(arr[6])  { if $test(arr[7]) {
                        if $test(arr[8])  { if $test(arr[9])  { if $test(arr[10]) { if $test(arr[11]) {
                        if $test(arr[12]) { if $test(arr[13]) { if $test(arr[14]) { if $test(arr[15]) {

                        $lex.bump(16); continue;     } $lex.bump(15); return $miss; }
                        $lex.bump(14); return $miss; } $lex.bump(13); return $miss; }
                        $lex.bump(12); return $miss; } $lex.bump(11); return $miss; }
                        $lex.bump(10); return $miss; } $lex.bump(9); return $miss;  }
                        $lex.bump(8); return $miss;  } $lex.bump(7); return $miss;  }
                        $lex.bump(6); return $miss;  } $lex.bump(5); return $miss;  }
                        $lex.bump(4); return $miss;  } $lex.bump(3); return $miss;  }
                        $lex.bump(2); return $miss;  } $lex.bump(1); return $miss;  }

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