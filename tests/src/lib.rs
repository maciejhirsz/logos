use logos::Logos;
use logos::source::Source;

use std::fmt;
use std::ops::Range;

// mod binary;

pub fn assert_lex<'a, Token>(
    source: &'a Token::Source,
    tokens: &[(Token, &'a <<Token as Logos>::Source as Source>::Slice, Range<usize>)],
) where
    Token: logos::Logos + fmt::Debug + PartialEq + Clone + Copy,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(&(lex.token, lex.slice(), lex.range()), tuple);

        lex.advance();
    }

    assert_eq!(lex.token, Token::END);
}
