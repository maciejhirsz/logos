# `#[logos]`

As previously said, the `#[logos]` attribute can be attached to the `enum`
of your token definition to add extras or define parts that must be skipped.

The syntax is as follows:

```rust,no_run,no_playground
#[derive(Logos)]
#[logos(skip r"<some string>")]
#[logos(extras = AnyType)]
enum Token {
    /* ... */
}
```

where `<some string>` can be any regex supported by [`#[regex]`](../common-regex,md),
and `AnyType` can be... Any type!

An example usage of `skip` is provided in the [JSON parser example](../examples/json.md).

For more details about extras, read the [eponym section](../extras.md).
