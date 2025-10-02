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
#[logos(source = SourceType)]
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

You can force the derive macro to use a different path to `Logos`'s crate
with `#[logos(crate = path::to::logos)]`.

## Custom source type

By default, **Logos**'s lexer will accept `&str` as input, unless any of the
pattern literals match a non utf-8 bytes sequence. In this case, it will fall
back to `&[u8]`. You can override this behavior by forcing one of the two
source types. You can also specify any custom type that implements
[`Source`](https://docs.rs/logos/latest/logos/source/trait.Source.html).

## Subpatterns

We can use subpatterns to reuse regular expressions in our tokens or other subpatterns.

The syntax tu use a previously defined subpattern, like `#[logos(subpattern subpattern_name = "regex literal")]`,
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
