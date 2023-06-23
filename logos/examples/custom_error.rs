//! ASCII tokens lexer with custom error type.
//!
//! Prints out tabs-or-spaces separated words,
//! only accepting ascii letters.
//!
//! Usage:
//!     cargo run --example custom_error

/* ANCHOR: all */
use logos::Logos;

#[derive(Default, Debug, Clone, PartialEq)]
enum LexingError {
    #[default]
    NonAsciiCharacter,
}

#[derive(Debug, Logos, PartialEq)]
#[logos(error = LexingError)]
#[logos(skip r"[ \t]+")]
enum Token {
    #[regex(r"[a-zA-Z]+")]
    Word,
}

fn main() {
    let mut lex = Token::lexer("I am Jérome");

    assert_eq!(lex.next(), Some(Ok(Token::Word)));
    assert_eq!(lex.slice(), "I");

    assert_eq!(lex.next(), Some(Ok(Token::Word)));
    assert_eq!(lex.slice(), "am");

    assert_eq!(lex.next(), Some(Ok(Token::Word)));
    assert_eq!(lex.slice(), "J");

    assert_eq!(lex.next(), Some(Err(LexingError::NonAsciiCharacter)));
    assert_eq!(lex.slice(), "é");
}
/* ANCHOR_END: all */
