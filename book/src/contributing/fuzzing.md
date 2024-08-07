# Fuzzing


Fuzzing is a technique to test a piece of software by injecting randomly generated inputs. This can be pretty useful to discover bugs, as pointed out in [#407](https://github.com/maciejhirsz/logos/pull/407).

**Logos**' fuzzing crate is powered by [afl.rs](https://github.com/rust-fuzz/afl.rs) that
finds panics in **Logos**' methods.

## Usage

Make sure you have `cargo-afl` installed. [See the rust-fuzz afl setup guide for installation information](https://rust-fuzz.github.io/book/afl/setup.html).

All of these commands assume your current directory is in the `fuzz` folder.

To build the fuzz target, run `cargo afl build`.

To start the fuzzer in the `fuzz` folder, run:

```sh
cargo afl fuzz -i in -o out ../target/debug/logos-fuzz
```

To replay any crashes, run:

```sh
cargo afl run logos-fuzz < out/default/crashes/crash_file
```

If you find a meaningful crash with a vague error message, send a PR to
help improve the developer experience of Logos.
