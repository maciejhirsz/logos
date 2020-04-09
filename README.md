<p align="center">
    <img src="https://raw.githubusercontent.com/maciejhirsz/logos/master/logos.png" width="60%" alt="Logos">
</p>

## Create ridiculously fast Lexers.

![Test](https://github.com/maciejhirsz/logos/workflows/Test/badge.svg?branch=master)
[![Crates.io version shield](https://img.shields.io/crates/v/logos.svg)](https://crates.io/crates/logos)
[![Crates.io license shield](https://img.shields.io/crates/l/logos.svg)](https://crates.io/crates/logos)

**Logos** works by:

+ Resolving all logical branching of token definitions into a state machine.
+ Optimizing complex patterns into [Lookup Tables](https://en.wikipedia.org/wiki/Lookup_table).
+ Avoiding backtracking, unwinding loops, and batching reads to minimize bounds checking.

In practice it means that for most grammars the lexing performance is virtually unaffected by the number
of tokens defined in the grammar. Or, in other words, **it is really fast**.

## Example

```rust
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
enum Token {
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    Error,

    // Tokens can be literal strings, of any length.
    #[token = "fast"]
    Fast,

    #[token = "."]
    Period,

    // Or regular expressions.
    #[regex = "[a-zA-Z]+"]
    Text,
}

fn main() {
    let mut lex = Token::lexer("Create ridiculously fast Lexers.");

    assert_eq!(lex.next(), Some(Token::Text));
    assert_eq!(lex.span(), 0..6);
    assert_eq!(lex.slice(), "Create");

    assert_eq!(lex.next(), Some(Token::Text));
    assert_eq!(lex.span(), 7..19);
    assert_eq!(lex.slice(), "ridiculously");

    assert_eq!(lex.next(), Some(Token::Fast));
    assert_eq!(lex.span(), 20..24);
    assert_eq!(lex.slice(), "fast");

    assert_eq!(lex.next(), Some(Token::Text));
    assert_eq!(lex.span(), 25..31);
    assert_eq!(lex.slice(), "Lexers");

    assert_eq!(lex.next(), Some(Token::Period));
    assert_eq!(lex.span(), 31..32);
    assert_eq!(lex.slice(), ".");

    assert_eq!(lex.next(), None);
}
```

### Callbacks

**Logos** can also call arbitrary functions whenever a pattern is matched,
which can be used to put data into a variant:

```rust
use logos::{Logos, Lexer, Extras};

// Note: callbacks can return `Option` or `Result`
fn kilo(lex: &mut Lexer<Token>) -> Option<u64> {
    let slice = lex.slice();
    let n: u64 = slice[..slice.len() - 1].parse().ok()?; // skip 'k'
    Some(n * 1_000)
}

fn mega(lex: &mut Lexer<Token>) -> Option<u64> {
    let slice = lex.slice();
    let n: u64 = slice[..slice.len() - 1].parse().ok()?; // skip 'm'
    Some(n * 1_000_000)
}

#[derive(Logos, Debug, PartialEq)]
enum Token {
    #[error]
    Error,

    // Callbacks can use closure syntax, or refer
    // to a function defined elsewhere.
    //
    // Each pattern can have it's own callback.
    #[regex("[0-9]+", |lex| lex.slice().parse())]
    #[regex("[0-9]+k", kilo)]
    #[regex("[0-9]+m", mega)]
    Number(u64),
}

fn main() {
    let mut lex = Token::lexer("5 42k 75m");

    assert_eq!(lex.next(), Some(Token::Number(5)));
    assert_eq!(lex.slice(), "5");

    assert_eq!(lex.next(), Some(Token::Number(42_000)));
    assert_eq!(lex.slice(), "42k");

    assert_eq!(lex.next(), Some(Token::Number(75_000_000)));
    assert_eq!(lex.slice(), "75m");

    assert_eq!(lex.next(), None);
}
```

Logos can handle callbacks with following return types:

| Return type     | Produces                                           |
|-----------------|----------------------------------------------------|
| `()`            | `Token::Unit`                                      |
| `bool`          | `Token::Unit` **or** `<Token as Logos>::ERROR`     |
| `Result<(), _>` | `Token::Unit` **or** `<Token as Logos>::ERROR`     |
| `T`             | `Token::Value(T)`                                  |
| `Option<T>`     | `Token::Value(T)` **or** `<Token as Logos>::ERROR` |
| `Result<T, _>`  | `Token::Value(T)` **or** `<Token as Logos>::ERROR` |

Callbacks can be also used to do perform more specialized lexing in place
where regular expressions are too limiting. For specifics look at
`Lexer::remainder` and `Lexer::bump`.

## Token disambiguation

Rule of thumb is:

+ Longer beats shorter.
+ Specific beats generic.

If any two definitions could match the same input, like `fast` and `[a-zA-Z]+`
in the example above, it's the longer and more specific definition of `Token::Fast`
that will be the result.

This is done by comparing numeric priority attached to each definition. Every consecutive,
non-repeating single byte adds 2 to the priority, while every range or regex class adds 1.
Loops or optional blocks are ignored, while alternations count the shortest alternative:

+ `[a-zA-Z]+` has a priority of 1 (lowest possible), because at minimum it can match a single byte to a class.
+ `foobar` has a priority of 12.
+ `(foo|hello)(bar)?` has a priority of 6, `foo` being it's shortest possible match.

## How fast?

Ridiculously fast!

```
test identifiers                       ... bench:         660 ns/iter (+/- 54) = 1180 MB/s
test keywords_operators_and_punctators ... bench:       2,033 ns/iter (+/- 69) = 1048 MB/s
test strings                           ... bench:         557 ns/iter (+/- 28) = 1563 MB/s
```

## License

This code is distributed under the terms of both the MIT license
and the Apache License (Version 2.0), choose whatever works for you.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
