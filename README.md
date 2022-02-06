<img src="https://raw.githubusercontent.com/maciejhirsz/logos/master/logos.svg?sanitize=true" alt="Logos logo" width="250" align="right">

# Logos

![Test](https://github.com/maciejhirsz/logos/workflows/Test/badge.svg?branch=master)
[![Crates.io version shield](https://img.shields.io/crates/v/logos.svg)](https://crates.io/crates/logos)
[![Docs](https://docs.rs/logos/badge.svg)](https://docs.rs/logos)
[![Crates.io license shield](https://img.shields.io/crates/l/logos.svg)](https://crates.io/crates/logos)

_Create ridiculously fast Lexers._

**Logos** has two goals:

+ To make it easy to create a Lexer, so you can focus on more complex problems.
+ To make the generated Lexer faster than anything you'd write by hand.

To achieve those, **Logos**:

+ Combines all token definitions into a single [deterministic state machine](https://en.wikipedia.org/wiki/Deterministic_finite_automaton).
+ Optimizes branches into [lookup tables](https://en.wikipedia.org/wiki/Lookup_table) or [jump tables](https://en.wikipedia.org/wiki/Branch_table).
+ Prevents [backtracking](https://en.wikipedia.org/wiki/ReDoS) inside token definitions.
+ [Unwinds loops](https://en.wikipedia.org/wiki/Loop_unrolling), and batches reads to minimize bounds checking.
+ Does all of that heavy lifting at compile time.

## Example

```rust
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
enum Token {
    // Tokens can be literal strings, of any length.
    #[token("fast")]
    Fast,

    #[token(".")]
    Period,

    // Or regular expressions.
    #[regex("[a-zA-Z]+")]
    Text,

    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
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
use logos::{Logos, Lexer};

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
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,

    // Callbacks can use closure syntax, or refer
    // to a function defined elsewhere.
    //
    // Each pattern can have it's own callback.
    #[regex("[0-9]+", |lex| lex.slice().parse())]
    #[regex("[0-9]+k", kilo)]
    #[regex("[0-9]+M", mega)]
    Number(u64),
}

fn main() {
    let mut lex = Token::lexer("5 42k 75M");

    assert_eq!(lex.next(), Some(Token::Number(5)));
    assert_eq!(lex.slice(), "5");

    assert_eq!(lex.next(), Some(Token::Number(42_000)));
    assert_eq!(lex.slice(), "42k");

    assert_eq!(lex.next(), Some(Token::Number(75_000_000)));
    assert_eq!(lex.slice(), "75M");

    assert_eq!(lex.next(), None);
}
```

Logos can handle callbacks with following return types:

| Return type                       | Produces                                           |
|-----------------------------------|----------------------------------------------------|
| `()`                              | `Token::Unit`                                      |
| `bool`                            | `Token::Unit` **or** `<Token as Logos>::ERROR`     |
| `Result<(), _>`                   | `Token::Unit` **or** `<Token as Logos>::ERROR`     |
| `T`                               | `Token::Value(T)`                                  |
| `Option<T>`                       | `Token::Value(T)` **or** `<Token as Logos>::ERROR` |
| `Result<T, _>`                    | `Token::Value(T)` **or** `<Token as Logos>::ERROR` |
| `Skip`                            | _skips matched input_                              |
| `Filter<T>`                       | `Token::Value(T)` **or** _skips matched input_     |

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

If two definitions compute to the same priority and can match the same input **Logos** will
fail to compile, point out the problematic definitions, and ask you to specify a manual
priority for either of them.

For example: `[abc]+` and `[cde]+` both can match sequences of `c`, and both have priority of 1.
Turning the first definition to `#[regex("[abc]+", priority = 2)]` will allow for tokens
to be disambiguated again, in this case all sequences of `c` will match `[abc]+`.

## How fast?

Ridiculously fast!

```norust
test identifiers                       ... bench:         647 ns/iter (+/- 27) = 1204 MB/s
test keywords_operators_and_punctators ... bench:       2,054 ns/iter (+/- 78) = 1037 MB/s
test strings                           ... bench:         553 ns/iter (+/- 34) = 1575 MB/s
```

## Acknowledgements

+ [Pedrors](https://pedrors.pt/) for the **Logos** logo.

## Thank you

**Logos** is very much a labor of love. If you find it useful, consider
[getting me some coffee](https://github.com/sponsors/maciejhirsz). ☕

## License

This code is distributed under the terms of both the MIT license
and the Apache License (Version 2.0), choose whatever works for you.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
