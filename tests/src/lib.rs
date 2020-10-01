use logos::Logos;
use logos::source::Source;

use std::fmt;
use std::ops::Range;

mod binary;

pub fn assert_lex<'a, Token>(
    source: &'a Token::Source,
    tokens: &[(Result<Token, Token::Error>, &'a <Token::Source as Source>::Slice, Range<usize>)],
) where
    Token: Logos<'a> + fmt::Debug + PartialEq,
    Token::Error: fmt::Debug + PartialEq,
    Token::Extras: Default,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(&(lex.next().expect("Unexpected end"), lex.slice(), lex.span()), tuple);
    }

    assert_eq!(lex.next(), None);
}
