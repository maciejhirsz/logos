# Simple array language

Programs written in [array languages](https://en.wikipedia.org/wiki/Array_programming) manipulate
arrays of values as their primary data. In this example, we create a simple one-dimensional[^1]
array language. Programs are interpreted as a sequence of instructions on an initially empty array.
Writing a number or variable appends it to the array, while sum (`+`) and product (`*`)
combine all the numbers in the array into a single value, making the array a singleton.

[^1]: Arrays can only contain numbers, not other arrays.

This example demonstrates how [explicit lifetime specification](../attributes/logos.md##Explicit_source_lifetime)
can be used to create lexers which output tokens with (non-static) lifetimes that outlive the source.
These lexers all share the same (immutable) state across different threads, without any cloning or `Arc`s!

## Example program

There is an example program you can run with[^2]:
```bash
cargo run --example array_language examples/array_program.txt
```

[^2]: You first need to clone [this repository](https://github.com/maciejhirsz/logos).

## Lexing

The variable environment maps variable names to values.

```rust,no_run,noplayground
{{#include ../../../examples/array_language.rs:environment}}
```

The token type is paremeterized by the lifetime `'a`, which is used in the lexer extras
as the lifetime of the borrow of the variable environment.

```rust,no_run,noplayground
{{#include ../../../examples/array_language.rs:tokens}}
```

The lexer uses two callbacks:

```rust,no_run,noplayground
{{#include ../../../examples/array_language.rs:callbacks}}
```

The `#[logos(lifetime = none)]` attribute explicitly specifies that `'a` is **not** the
source lifetime[^3]. This means that the borrow of the environment (and thus the tokens
the lexer produces) is independent of the source. Logos creates a `'s` lifetime for the
source instead, which is used in the error type to store the slice causing the error:

[^3]: Without the attribute, Logos will assume that `'a` is the source lifetime.

```rust,no_run,noplayground
{{#include ../../../examples/array_language.rs:error_type}}
```

A file is lexed by creating a separate lexer for each line of the file and combining the
results.

```rust,no_run,noplayground
{{#include ../../../examples/array_language.rs:lex_file}}
```

Scoped threads allow non-static borrows of variables outside the thread. Here, we use
this ability to store a borrow of the variable environment in the extras of each lexer.
This allows us to lex each line in parallel, sharing the variable environment between
threads without needing to clone it or wrap it in an `Arc`. Since the token lifetime
is independent of the source, the created tokens can be returned as is.

> [!NOTE]
>
> Lexing each line of a file in parallel is done as an example and is probably a bad idea
> in a real program. It's more likely that you would want to lex multiple *files* in
> parallel.

## Evaluation

Each token is evaluated by updating an accumulator, which starts empty.
A number or array token has its contents appended to the accumulator, whereas the sum
and product operators combine all the numbers in the accumulator into a singleton.
Once all tokens have been evaluated sequentially, the final accumulator is returned.

```rust,no_run,noplayground
{{#include ../../../examples/array_language.rs:evaluate}}
```

The lines in the input file are evaluated sequentially, printing the returned accumulator.

## Full code

```rust,no_run,noplayground
{{#include ../../../examples/array_language.rs:all}}
```