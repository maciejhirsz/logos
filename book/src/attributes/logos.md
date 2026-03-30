# `#[logos]`

As previously said, the `#[logos]` attribute can be attached to the `enum`
of your token definition to customize your lexer. Note that they all are
**optional**.

The syntax is as follows:

```rust,no_run,no_playground
#[derive(Logos)]
#[logos(skip "regex literal")]
#[logos(skip("regex literal"[, callback, priority = <integer>]))]
#[logos(extras = ExtrasType)]
#[logos(error = ErrorType)]
#[logos(crate = path::to::logos)]
#[logos(utf8 = true)]
#[logos(lifetime = 's)]
#[logos(subpattern subpattern_name = "regex literal")]
enum Token {
    /* ... */
}
```

where `"regex literal"` can be any regex supported by
[`#[regex]`](../common-regex.md), and `ExtrasType` can be of any type!

An example usage of `skip` is provided in the [JSON parser example](../examples/json.md).

For more details about extras, read the [eponym section](../extras.md).

## Custom error type

By default, **Logos** uses `()` as the error type, which means that it
doesn't store any information about the error.
This can be changed by using `#[logos(error = ErrorType)]` attribute on the enum.
The type `ErrorType` can be any type that implements `Clone`, `PartialEq`,
`Default` and `From<E>` for each callback's error type.

Here is an example using a custom error type:

```rust,no_run,noplayground
{{#include ../../../examples/custom_error.rs:all}}
```

You can add error variants to `LexingError`,
and implement `From<E>` for each error type `E` that could
be returned by a callback. See [callbacks](../callbacks.md).

`ErrorType` must implement the `Default` trait because invalid tokens, i.e.,
literals that do not match any variant, will produce `Err(ErrorType::default())`.

Alternatively, you can provide a callback with the alternate syntax
`#[logos(error(ErrorType, callback = ...))]`, which allows you to include information
from the lexer such as the span where the error occurred:

```rust,no_run,noplayground
#[derive(Logos)]
#[logos(error(Range<usize>, callback = |lex| lex.span()))]
enum Token {
    #[token("a")]
    A,
    #[token("b")]
    B,
}
```

## Specifying path to logos

You can force the derive macro to use a different path to `Logos`' crate
with `#[logos(crate = path::to::logos)]`.

## Custom source type

By default, **Logos**' lexer will accept `&str` as input. If any of the tokens
or regex patterns can match a non UTF-8 bytes sequence, this will cause a
compile-time error. In this case, you should supply `#[logos(utf8 = false)]`.
This will cause the lexer to accept a `&[u8]` instead.

In the past, you could also specify any custom type, but that feature has been removed.

## Explicit source lifetime

When the source lifetime is left unspecified, **Logos** will use the lifetime of the
token type as the source (in `enum Token<'a>`, `'a` will become the source lifetime).
The token lifetime is set to `'s` in the extras type, in concrete type declarations and
in callbacks when using an implicit source lifetime. When no lifetime is present on the
token type, a new `'s` lifetime is generated as the source.

You can specify the source lifetime explicitly using `#[logos(lifetime = 'a)]` to use
lifetime `'a` from the token lifetime parameters, or use `#[logos(lifetime = none)]` to
add a new source lifetime `'s` instead. Token lifetimes are not set to `'s` when using
an explicit source lifetime. If your token type has multiple lifetimes, the source
lifetime must be set explicitly.

Here is a small example using an explicit source lifetime:

```rust,no_run,noplayground
#[derive(Logos)]
#[logos(lifetime = 's)]
enum Foo<'s, 'a> {
    #[token("bar", |lex| lex.slice())]
    Bar(&'s str),
    #[token("baz", |_| "static")]
    Baz(&'a str),
}
```

For a more complete example, see the [array language example](../examples/array-language.md).

## Subpatterns

We can use subpatterns to reuse regular expressions in our tokens or other subpatterns.

The syntax to use a previously defined subpattern, like `#[logos(subpattern subpattern_name = "regex literal")]`,
in a new regular expression is `"(?&subpattern_name)"`.

For example:

```rust,no_run,noplayground
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"\s+")]
#[logos(subpattern alpha = r"[a-zA-Z]")]
#[logos(subpattern digit = r"[0-9]")]
#[logos(subpattern alphanum = r"(?&alpha)|(?&digit)")]
enum Token {
    #[regex("(?&alpha)+")]
    Word,
    #[regex("(?&digit)+")]
    Number,
    #[regex("(?&alphanum){2}")]
    TwoAlphanum,
    #[regex("(?&alphanum){3}")]
    ThreeAlphanum,
}

fn main() {
    let mut lex = Token::lexer("Word 1234 ab3 12");

    assert_eq!(lex.next(), Some(Ok(Token::Word)));
    assert_eq!(lex.slice(), "Word");

    assert_eq!(lex.next(), Some(Ok(Token::Number)));
    assert_eq!(lex.slice(), "1234");

    assert_eq!(lex.next(), Some(Ok(Token::ThreeAlphanum)));
    assert_eq!(lex.slice(), "ab3");

    assert_eq!(lex.next(), Some(Ok(Token::TwoAlphanum)));
    assert_eq!(lex.slice(), "12");

    assert_eq!(lex.next(), None);
}
```

(Note that the above subpatterns are redundant as the same can be achieved with [existing character classes](https://docs.rs/regex/latest/regex/#ascii-character-classes))
