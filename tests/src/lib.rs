use logos::source::Source;
use logos::Logos;

use std::fmt;
use std::ops::Range;

mod binary;

type LexInvariants<'a, Token> = (
    // the token itself
    Token,
    // the source corresponding to the token
    &'a <<Token as Logos<'a>>::Source as Source>::Slice,
    // the token's span
    Range<usize>,
);

pub fn assert_lex<'a, Token>(source: &'a Token::Source, tokens: &[LexInvariants<'a, Token>])
where
    Token: Logos<'a> + fmt::Debug + PartialEq,
    Token::Extras: Default,
{
    let mut lex = Token::lexer(source);

    for tuple in tokens {
        assert_eq!(
            &(lex.next().expect("Unexpected end"), lex.slice(), lex.span()),
            tuple
        );
    }

    assert_eq!(lex.next(), None);
}
