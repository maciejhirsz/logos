//! Pattern for handling incomplete input with an explicit token.
//!
//! Usage:
//!     cargo run --example partial_input

/* ANCHOR: all */
use logos::Logos;

#[derive(Debug, Logos, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
enum Token<'source> {
    #[regex(r"<[^>]*>", |lex| lex.slice())]
    Tag(&'source str),

    #[regex(r"<[^>]*")]
    IncompleteTag,

    #[regex(r"[^<\s]+", |lex| lex.slice())]
    Text(&'source str),
}

fn main() {
    let mut complete = Token::lexer("open <ready>");

    assert_eq!(complete.next(), Some(Ok(Token::Text("open"))));
    assert_eq!(complete.next(), Some(Ok(Token::Tag("<ready>"))));
    assert_eq!(complete.next(), None);

    let mut incomplete = Token::lexer("open <in-progress").spanned();

    assert_eq!(incomplete.next(), Some((Ok(Token::Text("open")), 0..4)));
    assert_eq!(incomplete.next(), Some((Ok(Token::IncompleteTag), 5..17)));
    assert_eq!(incomplete.next(), None);
}
/* ANCHOR_END: all */
