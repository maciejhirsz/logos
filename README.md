<p align="center">
    <img src="https://raw.githubusercontent.com/maciejhirsz/logos/master/logos.png" width="60%" alt="Logos">
</p>

## Create ridiculously fast Lexers.

[![Travis shield](https://travis-ci.org/maciejhirsz/logos.svg)](https://travis-ci.org/maciejhirsz/logos)
[![Crates.io version shield](https://img.shields.io/crates/v/logos.svg)](https://crates.io/crates/logos)
[![Crates.io license shield](https://img.shields.io/crates/l/logos.svg)](https://crates.io/crates/logos)

**Logos** works by:

+ Resolving all logical branching of token definitions into a state machine.
+ Optimizing complex patterns into [Lookup Tables](https://en.wikipedia.org/wiki/Lookup_table).
+ Avoiding backtracking, unwinding loops, and batching reads to minimize bounds checking.

In practice it means that for most grammars the lexing performance is virtually unaffected by the number
of tokens defined in the grammar. Or, in other words, **it is really fast**.

## Usage

```rust
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
enum Token {
    // Logos requires that we define two default variants,
    // one for end of input source,
    #[end]
    End,

    // ...and one for errors. Those can be named anything
    // you wish as long as the attributes are there.
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
    let mut lexer = Token::lexer("Create ridiculously fast Lexers.");

    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "Create");
    assert_eq!(lexer.range(), 0..6);

    lexer.advance();

    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "ridiculously");
    assert_eq!(lexer.range(), 7..19);

    lexer.advance();

    assert_eq!(lexer.token, Token::Fast);
    assert_eq!(lexer.slice(), "fast");
    assert_eq!(lexer.range(), 20..24);

    lexer.advance();

    assert_eq!(lexer.token, Token::Text);
    assert_eq!(lexer.slice(), "Lexers");
    assert_eq!(lexer.range(), 25..31);

    lexer.advance();

    assert_eq!(lexer.token, Token::Period);
    assert_eq!(lexer.slice(), ".");
    assert_eq!(lexer.range(), 31..32);

    lexer.advance();

    assert_eq!(lexer.token, Token::End);
}
```

### Callbacks

On top of using the enum variants, **Logos** can also call arbitrary functions whenever a pattern is matched:

```rust
use logos::{Logos, Lexer, Extras};

#[derive(Default)]
struct TokenExtras {
    denomination: u32,
}

impl Extras for TokenExtras {}

fn one<S>(lexer: &mut Lexer<Token, S>) {
    lexer.extras.denomination = 1;
}

fn kilo<S>(lexer: &mut Lexer<Token, S>) {
    lexer.extras.denomination = 1_000;
}

fn mega<S>(lexer: &mut Lexer<Token, S>) {
    lexer.extras.denomination = 1_000_000;
}

#[derive(Logos, Debug, PartialEq)]
#[extras = "TokenExtras"]
enum Token {
    #[end]
    End,

    #[error]
    Error,

    // You can apply multiple definitions to a single variant,
    // each with it's own callback.
    #[regex("[0-9]+", callback = "one")]
    #[regex("[0-9]+k", callback = "kilo")]
    #[regex("[0-9]+m", callback = "mega")]
    Number,
}

fn main() {
    let mut lexer = Token::lexer("5 42k 75m");

    assert_eq!(lexer.token, Token::Number);
    assert_eq!(lexer.slice(), "5");
    assert_eq!(lexer.extras.denomination, 1);

    lexer.advance();

    assert_eq!(lexer.token, Token::Number);
    assert_eq!(lexer.slice(), "42k");
    assert_eq!(lexer.extras.denomination, 1_000);

    lexer.advance();

    assert_eq!(lexer.token, Token::Number);
    assert_eq!(lexer.slice(), "75m");
    assert_eq!(lexer.extras.denomination, 1_000_000);

    lexer.advance();

    assert_eq!(lexer.token, Token::End);
}
```

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
test identifiers                       ... bench:         667 ns/iter (+/- 26) = 1167 MB/s
test keywords_operators_and_punctators ... bench:       1,984 ns/iter (+/- 105) = 1074 MB/s
test strings                           ... bench:         613 ns/iter (+/- 38) = 1420 MB/s
```

## License

This code is distributed under the terms of both the MIT license
and the Apache License (Version 2.0), choose whatever works for you.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
