use logos::Lexer;
use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[regex(r" ", |_| logos::Skip)]
    #[token(".")]
    Accessor,

    #[token("...")]
    Ellipsis,
}

#[test]
fn single_dot() {
    let mut lex = Lexer::<Token>::new_prefix(".");

    assert_eq!(lex.next(), None);
    assert_eq!(lex.span(), 0..0);
}

#[test]
fn single_dot_with_space() {
    let mut lex = Lexer::<Token>::new_prefix(". ");

    assert_eq!(lex.next(), Some(Ok(Token::Accessor)));
    assert_eq!(lex.next(), None);
    assert_eq!(lex.span(), 2..2);
}

#[test]
fn three_dots() {
    let mut lex = Lexer::<Token>::new_prefix("...");

    assert_eq!(lex.next(), Some(Ok(Token::Ellipsis)));
}
