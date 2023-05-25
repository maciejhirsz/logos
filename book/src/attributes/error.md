# `#[error]`

By default, **Logos** uses `()` as the error type, which means that it
doesn't store any information about the error.
This can be changed by using `#[logos(error = T)]` attribute on the enum.
The type `T` can be any type that implements `Clone`, `PartialEq`,
`Default` and `From<E>` for each callback's error type.

<!-- TODO: show code example with custom error -->
