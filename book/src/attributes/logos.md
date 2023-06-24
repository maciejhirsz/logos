# `#[logos]`

As previously said, the `#[logos]` attribute can be attached to the `enum`
of your token definition to customize your lexer. Note that they all are
**optional**.

The syntax is as follows:

```rust,no_run,no_playground
#[derive(Logos)]
#[logos(skip "regex literal")]
#[logos(extras = ExtrasType)]
#[logos(error = ErrorType)]
#[logos(crate = path::to::logos)]
#[logos(source = SourceType)]
enum Token {
    /* ... */
}
```

where `"regex literal"` can be any regex supported by
[`#[regex]`](../common-regex,md), and `ExtrasType` can be of any type!

An example usage of `skip` is provided in the [JSON parser example](../examples/json.md).

For more details about extras, read the [eponym section](../extras.md).

## Custom Error Type

By default, **Logos** uses `()` as the error type, which means that it
doesn't store any information about the error.
This can be changed by using `#[logos(error = ErrorType)]` attribute on the enum.
The type `ErrorType` can be any type that implements `Clone`, `PartialEq`,
`Default` and `From<E>` for each callback's error type.

`ErrorType` must implement the `Default` trait because invalid tokens, i.e.,
literals that do not match any variant, will produce `Err(ErrorType::default())`.

For example, here is an example using a custom error type:

```rust,no_run,noplayground
{{#include ../../../logos/examples/custom_error.rs:all}}
```

You can add error variants to `LexingError`,
and implement `From<E>` for each error type `E` that could
be returned by a callback. See [callbacks](../callbacks.md).

## Specifying Path to Logos

You can force the derive macro to use a different path to `Logos`'s crate
with `#[logos(crate = path::to::logos)]`.

## Custom Source Type

By default, **Logos**'s lexer will accept `&str` as input, unless any of the
pattern literals match a non utf-8 bytes sequence. In this case, it will fall
back to `&[u8]`. You can override this behavior by forcing one of the two
source types. In the future, it will be possible to use other types too.
