# Unsafe Code

By default, **Logos** uses unsafe code to avoid unnecessary bounds checks while
accessing slices of the input `Source`.

This unsafe code also exists in the code generated by the `Logos` derive macro,
which generates a deterministic finite automata (DFA). Reasoning about the correctness
of this generated code can be difficult - if the derivation of the DFA in `Logos`
is correct, then this generated code will be correct and any mistakes in implementation
would be caught given sufficient fuzz testing.

Use of unsafe code is the default as this typically provides the fastest parser.

## Disabling Unsafe Code

However, for applications accepting untrusted input in a trusted context, this
may not be a sufficient correctness justification.

For those applications which cannot tolerate unsafe code, the feature `forbid-unsafe`
may be enabled. This replaces unchecked accesses in the `Logos` crate with safe,
checked alternatives which will panic on out-of-bounds access rather than cause
undefined behavior. Additionally, code generated by the macro will not use the
unsafe keyword, so generated code may be used in a crates using the 
`#![forbid(unsafe_code)]` attribute.

When the `forbid-unsafe` feature is added to a direct dependency on the `Logos` crate,
[Feature Unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification)
ensures any transitive inclusion of `Logos` via other dependencies also have unsafe
code disabled.

Generally, disabling unsafe code will result in a slower parser.

However making definitive statements around performance of safe-only code is difficult,
as there are too many variables to consider between compiler optimizations,
the specific grammar being parsed, and the target processor. The automated benchmarks
of this crate show around a 10% slowdown in safe-only code at the time of this writing.
