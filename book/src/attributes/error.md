# `#[error]`

By default, **Logos** uses `()` as the error type, which means that it
doesn't store any information about the error.
This can be changed by using `#[logos(error = T)]` attribute on the enum.
The type `T` can be any type that implements `Clone`, `PartialEq`,
`Default` and `From<E>` for each callback's error type.

For example, here is an example using a custom error type:

```rust,no_run,noplayground
{{#include ../../../logos/examples/custom_error.rs:all}}
```

You can add error variants to `LexingError`, and implement `From<E>` for each error type `E` that could be returned by a callback. See [callbacks](../callbacks.md).
