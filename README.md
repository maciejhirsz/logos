<img src="https://raw.githubusercontent.com/maciejhirsz/logos/master/logos.svg?sanitize=true" alt="Logos logo" width="250" align="right">

# Logos

[![Book](https://github.com/maciejhirsz/logos/actions/workflows/pages.yml/badge.svg?branch=master)](https://logos.maciej.codes/)
[![Crates.io version shield](https://img.shields.io/crates/v/logos.svg)](https://crates.io/crates/logos)
[![Docs](https://docs.rs/logos/badge.svg)](https://docs.rs/logos)
[![Crates.io license shield](https://img.shields.io/crates/l/logos.svg)](https://crates.io/crates/logos)
[![Code coverage](https://codecov.io/gh/maciejhirsz/logos/branch/master/graph/badge.svg)](https://codecov.io/gh/maciejhirsz/logos)

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
#[logos(skip r"[ \t\n\f]+")] // Ignore this regex pattern between tokens
enum Token {
    // Tokens can be literal strings, of any length.
    #[token("fast")]
    Fast,

    #[token(".")]
    Period,

    // Or regular expressions.
    #[regex("[a-zA-Z]+")]
    Text,
}

fn main() {
    let mut lex = Token::lexer("Create ridiculously fast Lexers.");

    assert_eq!(lex.next(), Some(Ok(Token::Text)));
    assert_eq!(lex.span(), 0..6);
    assert_eq!(lex.slice(), "Create");

    assert_eq!(lex.next(), Some(Ok(Token::Text)));
    assert_eq!(lex.span(), 7..19);
    assert_eq!(lex.slice(), "ridiculously");

    assert_eq!(lex.next(), Some(Ok(Token::Fast)));
    assert_eq!(lex.span(), 20..24);
    assert_eq!(lex.slice(), "fast");

    assert_eq!(lex.next(), Some(Ok(Token::Text)));
    assert_eq!(lex.slice(), "Lexers");
    assert_eq!(lex.span(), 25..31);

    assert_eq!(lex.next(), Some(Ok(Token::Period)));
    assert_eq!(lex.span(), 31..32);
    assert_eq!(lex.slice(), ".");

    assert_eq!(lex.next(), None);
}
```

For more examples and documentation, please refer to the
[Logos handbook](https://maciejhirsz.github.io/logos/) or the
[crate documentation](https://docs.rs/logos/latest/logos/).

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

If you'd like to contribute to Logos, then consider reading the
[Contributing guide](https://maciejhirsz.github.io/logos/contributing).

## Contributing

**Logos** welcome any kind of contribution: bug reports, suggestions,
or new features!

Please use the
[issues](https://github.com/maciejhirsz/logos/issues) or
[pull requests](https://github.com/maciejhirsz/logos/pulls) tabs,
when appropriate.

To release a new version, follow the [RELEASE-PROCESS](RELEASE-PROCESS.md)

## License

This code is distributed under the terms of both the MIT license
and the Apache License (Version 2.0), choose whatever works for you.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
