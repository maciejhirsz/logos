<p align="center">
    <img src="https://raw.github.com/maciejhirsz/logos/master/logos.png?sanitize=true" width="60%" alt="Logos">
</p>

## Create ridiculously fast Lexers.

**Logos** works by:
+ Resolving all logical branching of token definitions into a tree.
+ Optimizing complex patterns into [Lookup Tables](https://en.wikipedia.org/wiki/Lookup_table).
+ Always using a Lookup Table for the first byte of a token.
+ Producing code that never backtracks, thus running at linear time or close to it.

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

## How fast?

Ridiculously fast!

```
test logos                ... bench:       2,005 ns/iter (+/- 16) = 1062 MB/s
test logos_nul_terminated ... bench:       1,828 ns/iter (+/- 69) = 1165 MB/s
```

## License

This code is distributed under the terms of both the MIT license
and the Apache License (Version 2.0), choose whatever works for you.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
